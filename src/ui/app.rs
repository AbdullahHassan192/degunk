use crossbeam_channel::Receiver;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::core::deleter::{delete_path, DeleteMode};
use crate::core::ecosystem::Ecosystem;
use crate::core::scanner::{DiscoveredArtifact, ScanMessage, Scanner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    SizeDesc,
    AgeDesc,
    NameAsc,
    EcosystemAsc,
}

impl SortMode {
    pub fn next(&self) -> Self {
        match self {
            SortMode::SizeDesc => SortMode::AgeDesc,
            SortMode::AgeDesc => SortMode::NameAsc,
            SortMode::NameAsc => SortMode::EcosystemAsc,
            SortMode::EcosystemAsc => SortMode::SizeDesc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletionState {
    Idle,
    Confirming,
    Deleting,
    Done { freed_bytes: u64, errors: usize },
}

pub struct App {
    pub roots: Vec<PathBuf>,
    pub allowed_ecosystems: Option<HashSet<Ecosystem>>,
    pub artifacts: Vec<DiscoveredArtifact>,
    pub selected_table_index: usize,
    pub is_scanning: bool,
    pub scanned_dirs_count: usize,
    pub spinner_tick: usize,
    pub search_query: String,
    pub is_searching: bool,
    pub sort_mode: SortMode,
    pub deletion_state: DeletionState,
    pub should_quit: bool,

    scanner_cancel: Option<Arc<AtomicBool>>,
    rx: Option<Receiver<ScanMessage>>,
}

impl App {
    pub fn new(roots: Vec<PathBuf>, allowed_ecosystems: Option<HashSet<Ecosystem>>) -> Self {
        let mut app = Self {
            roots,
            allowed_ecosystems,
            artifacts: Vec::new(),
            selected_table_index: 0,
            is_scanning: false,
            scanned_dirs_count: 0,
            spinner_tick: 0,
            search_query: String::new(),
            is_searching: false,
            sort_mode: SortMode::SizeDesc,
            deletion_state: DeletionState::Idle,
            should_quit: false,
            scanner_cancel: None,
            rx: None,
        };
        app.start_scan();
        app
    }

    pub fn start_scan(&mut self) {
        if let Some(ref cancel) = self.scanner_cancel {
            cancel.store(true, Ordering::Relaxed);
        }

        self.artifacts.clear();
        self.selected_table_index = 0;
        self.is_scanning = true;
        self.scanned_dirs_count = 0;

        let (tx, rx) = crossbeam_channel::unbounded();
        let scanner = Scanner::new();
        let cancel = scanner.cancel_handle();

        self.scanner_cancel = Some(cancel.clone());
        self.rx = Some(rx);

        Scanner::start_scan(
            self.roots.clone(),
            tx,
            cancel,
            self.allowed_ecosystems.clone(),
        );
    }

    pub fn tick(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);

        // Process incoming scan messages
        if let Some(ref rx) = self.rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanMessage::Found(art) => {
                        self.artifacts.push(art);
                    }
                    ScanMessage::SizeUpdated {
                        id,
                        size_bytes,
                        file_count,
                    } => {
                        if let Some(art) = self.artifacts.iter_mut().find(|a| a.id == id) {
                            art.size_bytes = size_bytes;
                            art.file_count = file_count;
                            art.size_calculated = true;
                        }
                    }
                    ScanMessage::ActivityUpdated { id, activity } => {
                        if let Some(art) = self.artifacts.iter_mut().find(|a| a.id == id) {
                            art.days_inactive = activity.days_inactive;
                            art.is_git = activity.is_git;
                            art.git_clean = activity.git_status == crate::core::git::GitStatus::Clean;
                            art.activity = Some(activity);
                        }
                    }
                    ScanMessage::Progress { scanned_dirs } => {
                        self.scanned_dirs_count = scanned_dirs;
                    }
                    ScanMessage::Finished { total_scanned_dirs } => {
                        self.scanned_dirs_count = total_scanned_dirs;
                        self.is_scanning = false;
                    }
                }
            }
        }
    }

    pub fn get_filtered_artifacts(&self) -> Vec<DiscoveredArtifact> {
        let q = self.search_query.to_lowercase();
        let mut items: Vec<DiscoveredArtifact> = self
            .artifacts
            .iter()
            .filter(|a| {
                if q.is_empty() {
                    true
                } else {
                    a.project_name.to_lowercase().contains(&q)
                        || a.folder_name.to_lowercase().contains(&q)
                        || a.ecosystem.name().to_lowercase().contains(&q)
                        || a.target_path.to_string_lossy().to_lowercase().contains(&q)
                }
            })
            .cloned()
            .collect();

        // Sort items
        match self.sort_mode {
            SortMode::SizeDesc => items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes)),
            SortMode::AgeDesc => items.sort_by(|a, b| b.days_inactive.cmp(&a.days_inactive)),
            SortMode::NameAsc => items.sort_by(|a, b| a.project_name.to_lowercase().cmp(&b.project_name.to_lowercase())),
            SortMode::EcosystemAsc => items.sort_by(|a, b| a.ecosystem.name().cmp(b.ecosystem.name())),
        }

        items
    }

    pub fn get_total_bytes(&self) -> u64 {
        self.artifacts
            .iter()
            .filter(|a| !a.is_deleted)
            .map(|a| a.size_bytes)
            .sum()
    }

    pub fn get_selected_stats(&self) -> (usize, u64) {
        let selected: Vec<_> = self
            .artifacts
            .iter()
            .filter(|a| a.is_selected && !a.is_deleted)
            .collect();

        let count = selected.len();
        let bytes = selected.iter().map(|a| a.size_bytes).sum();
        (count, bytes)
    }

    pub fn move_up(&mut self) {
        let count = self.get_filtered_artifacts().len();
        if count == 0 {
            self.selected_table_index = 0;
        } else if self.selected_table_index > 0 {
            self.selected_table_index -= 1;
        } else {
            self.selected_table_index = count - 1;
        }
    }

    pub fn move_down(&mut self) {
        let count = self.get_filtered_artifacts().len();
        if count == 0 {
            self.selected_table_index = 0;
        } else if self.selected_table_index + 1 < count {
            self.selected_table_index += 1;
        } else {
            self.selected_table_index = 0;
        }
    }

    pub fn toggle_selection(&mut self) {
        let filtered = self.get_filtered_artifacts();
        if let Some(target) = filtered.get(self.selected_table_index) {
            let target_id = target.id;
            if let Some(art) = self.artifacts.iter_mut().find(|a| a.id == target_id) {
                if !art.is_deleted {
                    art.is_selected = !art.is_selected;
                }
            }
        }
    }

    pub fn toggle_all(&mut self) {
        let filtered = self.get_filtered_artifacts();
        let any_selected = filtered.iter().any(|a| a.is_selected && !a.is_deleted);
        let target_ids: HashSet<usize> = filtered.iter().map(|a| a.id).collect();

        for art in &mut self.artifacts {
            if target_ids.contains(&art.id) && !art.is_deleted {
                art.is_selected = !any_selected;
            }
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
    }

    pub fn perform_deletion(&mut self, mode: DeleteMode) {
        self.deletion_state = DeletionState::Deleting;

        let mut freed_bytes = 0u64;
        let mut errors = 0usize;

        for art in &mut self.artifacts {
            if art.is_selected && !art.is_deleted {
                match delete_path(&art.target_path, mode) {
                    Ok(()) => {
                        art.is_deleted = true;
                        art.is_selected = false;
                        freed_bytes += art.size_bytes;
                    }
                    Err(_) => {
                        errors += 1;
                    }
                }
            }
        }

        self.deletion_state = DeletionState::Done {
            freed_bytes,
            errors,
        };
    }
}
