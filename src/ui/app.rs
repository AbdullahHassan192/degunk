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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupSelectionState {
    All,
    Partial,
    None,
    AllDeleted,
}

#[derive(Debug, Clone)]
pub enum TableItem {
    GroupHeader {
        group_key: String,
        display_name: String,
        primary_ecosystem: Ecosystem,
        artifact_count: usize,
        total_bytes: u64,
        days_inactive: u32,
        activity: Option<crate::core::git::ProjectActivity>,
        all_locked: bool,
        is_expanded: bool,
        selection_state: GroupSelectionState,
        all_deleted: bool,
    },
    ChildArtifact {
        group_key: String,
        artifact_id: usize,
        display_label: String,
        folder_name: String,
        ecosystem: Ecosystem,
        size_bytes: u64,
        size_calculated: bool,
        has_lockfile: bool,
        lockfile_name: Option<String>,
        days_inactive: u32,
        activity: Option<crate::core::git::ProjectActivity>,
        is_selected: bool,
        is_deleted: bool,
        is_last: bool,
    },
}

pub struct App {
    pub roots: Vec<PathBuf>,
    pub allowed_ecosystems: Option<HashSet<Ecosystem>>,
    pub include_cloud: bool,
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
    pub collapsed_groups: HashSet<String>,

    scanner_cancel: Option<Arc<AtomicBool>>,
    rx: Option<Receiver<ScanMessage>>,
}

impl App {
    pub fn new(
        roots: Vec<PathBuf>,
        allowed_ecosystems: Option<HashSet<Ecosystem>>,
        include_cloud: bool,
    ) -> Self {
        let mut app = Self {
            roots,
            allowed_ecosystems,
            include_cloud,
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
            collapsed_groups: HashSet::new(),
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
            self.include_cloud,
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

    pub fn get_visible_table_items(&self) -> Vec<TableItem> {
        let q = self.search_query.to_lowercase();

        // Filter artifacts by search query
        let filtered_artifacts: Vec<&DiscoveredArtifact> = self
            .artifacts
            .iter()
            .filter(|a| {
                if q.is_empty() {
                    true
                } else {
                    a.root_project_name.to_lowercase().contains(&q)
                        || a.project_name.to_lowercase().contains(&q)
                        || a.display_path.to_lowercase().contains(&q)
                        || a.folder_name.to_lowercase().contains(&q)
                        || a.ecosystem.name().to_lowercase().contains(&q)
                        || a.target_path.to_string_lossy().to_lowercase().contains(&q)
                }
            })
            .collect();

        // Group filtered artifacts by root_project_name (or display_path)
        let mut group_map: std::collections::BTreeMap<String, Vec<&DiscoveredArtifact>> =
            std::collections::BTreeMap::new();
        for a in filtered_artifacts {
            let key = if a.root_project_name.is_empty() {
                a.display_path.clone()
            } else {
                a.root_project_name.clone()
            };
            group_map.entry(key).or_default().push(a);
        }

        struct GroupAggregate<'a> {
            group_key: String,
            display_name: String,
            primary_ecosystem: Ecosystem,
            total_bytes: u64,
            days_inactive: u32,
            activity: Option<crate::core::git::ProjectActivity>,
            all_locked: bool,
            selection_state: GroupSelectionState,
            all_deleted: bool,
            is_expanded: bool,
            items: Vec<&'a DiscoveredArtifact>,
        }

        let mut groups: Vec<GroupAggregate> = group_map
            .into_iter()
            .map(|(key, mut items)| {
                items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

                let total_bytes: u64 = items
                    .iter()
                    .filter(|a| !a.is_deleted)
                    .map(|a| a.size_bytes)
                    .sum();
                let all_deleted = items.iter().all(|a| a.is_deleted);

                let active_items: Vec<_> = items.iter().filter(|a| !a.is_deleted).collect();
                let selection_state = if all_deleted {
                    GroupSelectionState::AllDeleted
                } else if !active_items.is_empty() && active_items.iter().all(|a| a.is_selected) {
                    GroupSelectionState::All
                } else if active_items.iter().any(|a| a.is_selected) {
                    GroupSelectionState::Partial
                } else {
                    GroupSelectionState::None
                };

                let primary_ecosystem =
                    items.first().map(|a| a.ecosystem).unwrap_or(Ecosystem::Node);
                let days_inactive = items.iter().map(|a| a.days_inactive).max().unwrap_or(0);
                let activity = items.iter().find_map(|a| a.activity.clone());
                let all_locked = items.iter().all(|a| a.has_lockfile);

                let is_expanded = if !q.is_empty() {
                    true
                } else {
                    !self.collapsed_groups.contains(&key)
                };

                GroupAggregate {
                    group_key: key.clone(),
                    display_name: key,
                    primary_ecosystem,
                    total_bytes,
                    days_inactive,
                    activity,
                    all_locked,
                    selection_state,
                    all_deleted,
                    is_expanded,
                    items,
                }
            })
            .collect();

        // Sort groups
        match self.sort_mode {
            SortMode::SizeDesc => groups.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes)),
            SortMode::AgeDesc => groups.sort_by(|a, b| b.days_inactive.cmp(&a.days_inactive)),
            SortMode::NameAsc => groups.sort_by(|a, b| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            }),
            SortMode::EcosystemAsc => {
                groups.sort_by(|a, b| a.primary_ecosystem.name().cmp(b.primary_ecosystem.name()))
            }
        }

        // Flatten into visible table rows
        let mut visible_items = Vec::new();

        for g in groups {
            let artifact_count = g.items.len();
            let is_expanded = g.is_expanded;

            visible_items.push(TableItem::GroupHeader {
                group_key: g.group_key.clone(),
                display_name: g.display_name,
                primary_ecosystem: g.primary_ecosystem,
                artifact_count,
                total_bytes: g.total_bytes,
                days_inactive: g.days_inactive,
                activity: g.activity,
                all_locked: g.all_locked,
                is_expanded,
                selection_state: g.selection_state,
                all_deleted: g.all_deleted,
            });

            if is_expanded {
                let total_children = g.items.len();
                for (idx, item) in g.items.into_iter().enumerate() {
                    let is_last = idx + 1 == total_children;
                    let rel_label = if item.sub_path.is_empty() {
                        item.folder_name.clone()
                    } else {
                        format!("{}/{}", item.sub_path, item.folder_name)
                    };

                    visible_items.push(TableItem::ChildArtifact {
                        group_key: g.group_key.clone(),
                        artifact_id: item.id,
                        display_label: rel_label,
                        folder_name: item.folder_name.clone(),
                        ecosystem: item.ecosystem,
                        size_bytes: item.size_bytes,
                        size_calculated: item.size_calculated,
                        has_lockfile: item.has_lockfile,
                        lockfile_name: item.lockfile_name.clone(),
                        days_inactive: item.days_inactive,
                        activity: item.activity.clone(),
                        is_selected: item.is_selected,
                        is_deleted: item.is_deleted,
                        is_last,
                    });
                }
            }
        }

        visible_items
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
                        || a.display_path.to_lowercase().contains(&q)
                        || a.folder_name.to_lowercase().contains(&q)
                        || a.ecosystem.name().to_lowercase().contains(&q)
                        || a.target_path.to_string_lossy().to_lowercase().contains(&q)
                }
            })
            .cloned()
            .collect();

        match self.sort_mode {
            SortMode::SizeDesc => items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes)),
            SortMode::AgeDesc => items.sort_by(|a, b| b.days_inactive.cmp(&a.days_inactive)),
            SortMode::NameAsc => items.sort_by(|a, b| {
                a.project_name
                    .to_lowercase()
                    .cmp(&b.project_name.to_lowercase())
            }),
            SortMode::EcosystemAsc => {
                items.sort_by(|a, b| a.ecosystem.name().cmp(b.ecosystem.name()))
            }
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
        let count = self.get_visible_table_items().len();
        if count == 0 {
            self.selected_table_index = 0;
        } else if self.selected_table_index > 0 {
            self.selected_table_index -= 1;
        } else {
            self.selected_table_index = count.saturating_sub(1);
        }
    }

    pub fn move_down(&mut self) {
        let count = self.get_visible_table_items().len();
        if count == 0 {
            self.selected_table_index = 0;
        } else if self.selected_table_index + 1 < count {
            self.selected_table_index += 1;
        } else {
            self.selected_table_index = 0;
        }
    }

    pub fn toggle_selection(&mut self) {
        let visible = self.get_visible_table_items();
        if let Some(target) = visible.get(self.selected_table_index) {
            match target {
                TableItem::GroupHeader {
                    group_key,
                    selection_state,
                    ..
                } => {
                    let should_select = *selection_state != GroupSelectionState::All;
                    for art in &mut self.artifacts {
                        let art_key = if art.root_project_name.is_empty() {
                            &art.display_path
                        } else {
                            &art.root_project_name
                        };
                        if art_key == group_key && !art.is_deleted {
                            art.is_selected = should_select;
                        }
                    }
                }
                TableItem::ChildArtifact { artifact_id, .. } => {
                    let id = *artifact_id;
                    if let Some(art) = self.artifacts.iter_mut().find(|a| a.id == id) {
                        if !art.is_deleted {
                            art.is_selected = !art.is_selected;
                        }
                    }
                }
            }
        }
    }

    pub fn toggle_expand(&mut self) {
        let visible = self.get_visible_table_items();
        if let Some(target) = visible.get(self.selected_table_index) {
            match target {
                TableItem::GroupHeader {
                    group_key,
                    is_expanded,
                    ..
                } => {
                    if *is_expanded {
                        self.collapsed_groups.insert(group_key.clone());
                    } else {
                        self.collapsed_groups.remove(group_key);
                    }
                }
                TableItem::ChildArtifact { group_key, .. } => {
                    let key = group_key.clone();
                    if let Some(header_idx) = visible.iter().position(|it| {
                        matches!(it, TableItem::GroupHeader { group_key: k, .. } if k == &key)
                    }) {
                        self.selected_table_index = header_idx;
                    }
                }
            }
        }
    }

    pub fn expand_group(&mut self) {
        let visible = self.get_visible_table_items();
        if let Some(target) = visible.get(self.selected_table_index) {
            match target {
                TableItem::GroupHeader { group_key, .. } => {
                    self.collapsed_groups.remove(group_key);
                }
                TableItem::ChildArtifact { .. } => {}
            }
        }
    }

    pub fn collapse_group(&mut self) {
        let visible = self.get_visible_table_items();
        if let Some(target) = visible.get(self.selected_table_index) {
            match target {
                TableItem::GroupHeader { group_key, .. } => {
                    self.collapsed_groups.insert(group_key.clone());
                }
                TableItem::ChildArtifact { group_key, .. } => {
                    let key = group_key.clone();
                    if let Some(header_idx) = visible.iter().position(|it| {
                        matches!(it, TableItem::GroupHeader { group_key: k, .. } if k == &key)
                    }) {
                        self.selected_table_index = header_idx;
                    }
                }
            }
        }
    }

    pub fn toggle_expand_all(&mut self) {
        let visible = self.get_visible_table_items();
        let any_expanded = visible.iter().any(|it| {
            matches!(
                it,
                TableItem::GroupHeader {
                    is_expanded: true,
                    ..
                }
            )
        });

        if any_expanded {
            for it in visible {
                if let TableItem::GroupHeader { group_key, .. } = it {
                    self.collapsed_groups.insert(group_key);
                }
            }
        } else {
            self.collapsed_groups.clear();
        }
    }

    pub fn toggle_all(&mut self) {
        let any_selected = self.artifacts.iter().any(|a| a.is_selected && !a.is_deleted);
        for art in &mut self.artifacts {
            if !art.is_deleted {
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
