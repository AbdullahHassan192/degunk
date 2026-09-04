use crossbeam_channel::Receiver;
use ratatui::widgets::TableState;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use crate::core::deleter::{
    delete_path_with_progress, get_error_log_path, log_deletion_errors, DeleteMode,
    DeleteProgressMessage, DeletionTargetError,
};
use crate::core::ecosystem::Ecosystem;
use crate::core::global_cache::{
    detect_global_caches, start_global_cache_scan, GlobalCacheMessage, GlobalCacheTarget,
};
use crate::core::paths::detect_suggested_paths;
use crate::core::scanner::{DiscoveredArtifact, ScanMessage, Scanner};
use crate::core::size::parse_size_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Projects,
    GlobalCaches,
}

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
    Deleting {
        current_target: usize,
        total_targets: usize,
        completed_targets: usize,
        current_path: String,
        freed_bytes: u64,
        total_bytes: u64,
        mode: DeleteMode,
    },
    Done {
        freed_bytes: u64,
        errors: usize,
        mode: DeleteMode,
        cancelled: bool,
        error_details: Vec<String>,
        log_path: Option<PathBuf>,
    },
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
        is_deleting: bool,
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
        is_deleting: bool,
        is_last: bool,
    },
}

#[derive(Debug, Clone)]
pub struct PathPickerItem {
    pub label: String,
    pub path_display: String,
    pub target: PathPickerTarget,
    pub shortcut: Option<char>,
}

#[derive(Debug, Clone)]
pub enum PathPickerTarget {
    Path(PathBuf),
    CustomInput,
    GlobalCaches,
}

#[derive(Debug, Clone)]
pub struct PathPickerState {
    pub items: Vec<PathPickerItem>,
    pub selected_index: usize,
    pub is_entering_custom: bool,
    pub custom_input: String,
    pub custom_error: Option<String>,
}

impl PathPickerState {
    pub fn new() -> Self {
        let suggested = detect_suggested_paths();
        let mut items = Vec::new();
        let mut quick_pick_num = 1;

        // 1. Current working directory
        if let Some(cwd) = suggested
            .iter()
            .find(|s| s.kind == crate::core::paths::PathKind::CurrentDir)
        {
            let shortcut = if quick_pick_num <= 9 {
                let ch = char::from_digit(quick_pick_num as u32, 10);
                quick_pick_num += 1;
                ch
            } else {
                None
            };

            items.push(PathPickerItem {
                label: cwd.label.clone(),
                path_display: cwd.path.display().to_string(),
                target: PathPickerTarget::Path(cwd.path.clone()),
                shortcut,
            });
        }

        // 2. Custom Directory directly beneath Current Directory
        let custom_shortcut = if quick_pick_num <= 9 {
            let ch = char::from_digit(quick_pick_num as u32, 10);
            quick_pick_num += 1;
            ch
        } else {
            None
        };

        items.push(PathPickerItem {
            label: "Custom Directory".to_string(),
            path_display: "Enter or paste any path...".to_string(),
            target: PathPickerTarget::CustomInput,
            shortcut: custom_shortcut,
        });

        // 3. Dev folders and drives
        for s in suggested
            .iter()
            .filter(|s| s.kind != crate::core::paths::PathKind::CurrentDir)
        {
            let shortcut = if quick_pick_num <= 9 {
                let ch = char::from_digit(quick_pick_num as u32, 10);
                quick_pick_num += 1;
                ch
            } else {
                None
            };

            items.push(PathPickerItem {
                label: s.label.clone(),
                path_display: s.path.display().to_string(),
                target: PathPickerTarget::Path(s.path.clone()),
                shortcut,
            });
        }

        // 4. Global Tool Caches
        let global_shortcut = if quick_pick_num <= 9 {
            char::from_digit(quick_pick_num as u32, 10)
        } else {
            None
        };

        items.push(PathPickerItem {
            label: "Global Tool Caches".to_string(),
            path_display: "View central caches (Cargo, npm, Ollama...) without scanning folders".to_string(),
            target: PathPickerTarget::GlobalCaches,
            shortcut: global_shortcut,
        });

        Self {
            items,
            selected_index: 0,
            is_entering_custom: false,
            custom_input: String::new(),
            custom_error: None,
        }
    }
}

pub struct App {
    pub roots: Vec<PathBuf>,
    pub allowed_ecosystems: Option<HashSet<Ecosystem>>,
    pub include_cloud: bool,
    pub has_scanned: bool,
    pub path_picker: Option<PathPickerState>,
    pub artifacts: Vec<DiscoveredArtifact>,
    pub selected_table_index: usize,
    pub table_state: TableState,
    pub is_scanning: bool,
    pub scanned_dirs_count: usize,
    pub spinner_tick: usize,
    pub search_query: String,
    pub is_searching: bool,
    pub sort_mode: SortMode,
    pub deletion_state: DeletionState,
    pub deleting_paths: HashSet<PathBuf>,
    pub should_quit: bool,
    pub collapsed_groups: HashSet<String>,

    pub active_tab: ActiveTab,
    pub global_caches: Vec<GlobalCacheTarget>,
    pub selected_cache_index: usize,
    pub cache_table_state: TableState,

    pub deletion_cancel: Option<Arc<AtomicBool>>,

    scanner_cancel: Option<Arc<AtomicBool>>,
    rx: Option<Receiver<ScanMessage>>,
    global_cache_cancel: Option<Arc<AtomicBool>>,
    global_cache_rx: Option<Receiver<GlobalCacheMessage>>,
    deletion_rx: Option<Receiver<DeleteProgressMessage>>,
}

impl App {
    pub fn new(
        roots: Vec<PathBuf>,
        allowed_ecosystems: Option<HashSet<Ecosystem>>,
        include_cloud: bool,
        has_explicit_paths: bool,
    ) -> Self {
        let path_picker = if has_explicit_paths {
            None
        } else {
            Some(PathPickerState::new())
        };

        let mut app = Self {
            roots,
            allowed_ecosystems,
            include_cloud,
            has_scanned: has_explicit_paths,
            path_picker,
            artifacts: Vec::new(),
            selected_table_index: 0,
            table_state: TableState::default(),
            is_scanning: false,
            scanned_dirs_count: 0,
            spinner_tick: 0,
            search_query: String::new(),
            is_searching: false,
            sort_mode: SortMode::SizeDesc,
            deletion_state: DeletionState::Idle,
            deleting_paths: HashSet::new(),
            should_quit: false,
            collapsed_groups: HashSet::new(),
            active_tab: ActiveTab::Projects,
            global_caches: Vec::new(),
            selected_cache_index: 0,
            cache_table_state: TableState::default(),
            deletion_cancel: None,
            scanner_cancel: None,
            rx: None,
            global_cache_cancel: None,
            global_cache_rx: None,
            deletion_rx: None,
        };

        app.start_global_cache_scan();

        if has_explicit_paths {
            app.start_scan();
        }

        app
    }

    pub fn start_global_cache_scan(&mut self) {
        if let Some(ref cancel) = self.global_cache_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        self.global_caches = detect_global_caches();
        self.selected_cache_index = 0;
        self.cache_table_state = TableState::default();
        self.cache_table_state.select(Some(0));

        let (gc_tx, gc_rx) = crossbeam_channel::unbounded();
        let gc_cancel = Arc::new(AtomicBool::new(false));
        self.global_cache_cancel = Some(gc_cancel.clone());
        self.global_cache_rx = Some(gc_rx);
        start_global_cache_scan(self.global_caches.clone(), gc_tx, gc_cancel);
    }

    pub fn open_path_picker(&mut self) {
        self.path_picker = Some(PathPickerState::new());
    }

    pub fn select_path(&mut self, path: PathBuf) {
        self.roots = vec![path];
        self.path_picker = None;
        self.active_tab = ActiveTab::Projects;
        self.has_scanned = true;
        self.start_scan();
    }

    pub fn start_scan(&mut self) {
        if let Some(ref cancel) = self.scanner_cancel {
            cancel.store(true, Ordering::Relaxed);
        }

        self.artifacts.clear();
        self.selected_table_index = 0;
        self.table_state = TableState::default();
        self.table_state.select(Some(0));
        self.is_scanning = true;
        self.scanned_dirs_count = 0;
        self.has_scanned = true;

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

    pub fn cancel_deletion(&mut self) {
        if let Some(ref cancel) = self.deletion_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
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

        // Process incoming global cache sizing messages
        if let Some(ref rx) = self.global_cache_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    GlobalCacheMessage::Discovered(targets) => {
                        self.global_caches = targets;
                    }
                    GlobalCacheMessage::SizeUpdated {
                        id,
                        size_bytes,
                        file_count,
                    } => {
                        if let Some(cache) = self.global_caches.iter_mut().find(|c| c.id == id) {
                            cache.size_bytes = size_bytes;
                            cache.file_count = file_count;
                            cache.size_calculated = true;
                        }
                    }
                    GlobalCacheMessage::Finished => {}
                }
            }
        }

        // Process non-blocking deletion progress
        let mut deletion_finished = false;
        if let Some(ref rx) = self.deletion_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    DeleteProgressMessage::Progress {
                        current_target,
                        total_targets,
                        completed_targets,
                        current_path,
                        freed_bytes,
                        total_bytes,
                        mode,
                    } => {
                        self.deletion_state = DeletionState::Deleting {
                            current_target,
                            total_targets,
                            completed_targets,
                            current_path,
                            freed_bytes,
                            total_bytes,
                            mode,
                        };
                    }
                    DeleteProgressMessage::TargetFinished {
                        path,
                        success,
                        freed_bytes,
                    } => {
                        self.deleting_paths.remove(&path);
                        if success {
                            for art in &mut self.artifacts {
                                if art.target_path == path {
                                    art.is_deleted = true;
                                    art.is_selected = false;
                                }
                            }
                            for cache in &mut self.global_caches {
                                if cache.path == path {
                                    cache.is_deleted = true;
                                    cache.is_selected = false;
                                }
                            }
                        } else if freed_bytes > 0 {
                            for art in &mut self.artifacts {
                                if art.target_path == path {
                                    art.size_bytes = art.size_bytes.saturating_sub(freed_bytes);
                                }
                            }
                            for cache in &mut self.global_caches {
                                if cache.path == path {
                                    cache.size_bytes = cache.size_bytes.saturating_sub(freed_bytes);
                                }
                            }
                        }
                    }
                    DeleteProgressMessage::Done {
                        freed_bytes,
                        errors,
                        mode,
                        cancelled,
                        error_details,
                        log_path,
                    } => {
                        self.deleting_paths.clear();
                        self.deletion_state = DeletionState::Done {
                            freed_bytes,
                            errors,
                            mode,
                            cancelled,
                            error_details,
                            log_path,
                        };
                        self.deletion_cancel = None;
                        deletion_finished = true;
                    }
                }
            }
        }
        if deletion_finished {
            self.deletion_rx = None;
        }
    }

    pub fn get_visible_table_items(&self) -> Vec<TableItem> {
        let q = self.search_query.trim();

        // Filter artifacts by search query
        let filtered_artifacts: Vec<&DiscoveredArtifact> = self
            .artifacts
            .iter()
            .filter(|a| matches_artifact_query(a, q))
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
            is_deleting: bool,
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
                let is_deleting = items.iter().any(|a| self.deleting_paths.contains(&a.target_path));

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
                    is_deleting,
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
                is_deleting: g.is_deleting,
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

                    let is_deleting = self.deleting_paths.contains(&item.target_path);

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
                        is_deleting,
                        is_last,
                    });
                }
            }
        }

        visible_items
    }

    pub fn get_filtered_artifacts(&self) -> Vec<DiscoveredArtifact> {
        let q = self.search_query.trim();
        let mut items: Vec<DiscoveredArtifact> = self
            .artifacts
            .iter()
            .filter(|a| matches_artifact_query(a, q))
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

    pub fn get_visible_global_caches(&self) -> Vec<(usize, &GlobalCacheTarget)> {
        let mut items: Vec<(usize, &GlobalCacheTarget)> = self
            .global_caches
            .iter()
            .enumerate()
            .filter(|(_, c)| matches_global_cache_query(c, &self.search_query))
            .collect();

        match self.sort_mode {
            SortMode::SizeDesc => items.sort_by(|(_, a), (_, b)| b.size_bytes.cmp(&a.size_bytes)),
            SortMode::AgeDesc => items.sort_by(|(_, a), (_, b)| b.file_count.cmp(&a.file_count)),
            SortMode::NameAsc => items.sort_by(|(_, a), (_, b)| {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }),
            SortMode::EcosystemAsc => {
                items.sort_by(|(_, a), (_, b)| a.ecosystem.name().cmp(b.ecosystem.name()))
            }
        }

        items
    }

    pub fn switch_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Projects => ActiveTab::GlobalCaches,
            ActiveTab::GlobalCaches => ActiveTab::Projects,
        };
    }

    pub fn get_total_bytes(&self) -> u64 {
        match self.active_tab {
            ActiveTab::Projects => self.get_total_project_bytes(),
            ActiveTab::GlobalCaches => self.get_total_global_cache_bytes(),
        }
    }

    pub fn get_total_project_bytes(&self) -> u64 {
        self.artifacts
            .iter()
            .filter(|a| !a.is_deleted)
            .map(|a| a.size_bytes)
            .sum()
    }

    pub fn get_total_global_cache_bytes(&self) -> u64 {
        self.global_caches
            .iter()
            .filter(|c| !c.is_deleted)
            .map(|c| c.size_bytes)
            .sum()
    }

    pub fn get_selected_stats(&self) -> (usize, u64) {
        match self.active_tab {
            ActiveTab::Projects => {
                let selected: Vec<_> = self
                    .artifacts
                    .iter()
                    .filter(|a| a.is_selected && !a.is_deleted)
                    .collect();
                let count = selected.len();
                let bytes = selected.iter().map(|a| a.size_bytes).sum();
                (count, bytes)
            }
            ActiveTab::GlobalCaches => {
                let selected: Vec<_> = self
                    .global_caches
                    .iter()
                    .filter(|c| c.is_selected && !c.is_deleted)
                    .collect();
                let count = selected.len();
                let bytes = selected.iter().map(|c| c.size_bytes).sum();
                (count, bytes)
            }
        }
    }

    pub fn move_up(&mut self) {
        match self.active_tab {
            ActiveTab::Projects => {
                let count = self.get_visible_table_items().len();
                if count == 0 {
                    self.selected_table_index = 0;
                } else if self.selected_table_index > 0 {
                    self.selected_table_index -= 1;
                } else {
                    self.selected_table_index = count.saturating_sub(1);
                }
                self.table_state.select(Some(self.selected_table_index));
            }
            ActiveTab::GlobalCaches => {
                let count = self.get_visible_global_caches().len();
                if count == 0 {
                    self.selected_cache_index = 0;
                } else if self.selected_cache_index > 0 {
                    self.selected_cache_index -= 1;
                } else {
                    self.selected_cache_index = count.saturating_sub(1);
                }
                self.cache_table_state.select(Some(self.selected_cache_index));
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.active_tab {
            ActiveTab::Projects => {
                let count = self.get_visible_table_items().len();
                if count == 0 {
                    self.selected_table_index = 0;
                } else if self.selected_table_index + 1 < count {
                    self.selected_table_index += 1;
                } else {
                    self.selected_table_index = 0;
                }
                self.table_state.select(Some(self.selected_table_index));
            }
            ActiveTab::GlobalCaches => {
                let count = self.get_visible_global_caches().len();
                if count == 0 {
                    self.selected_cache_index = 0;
                } else if self.selected_cache_index + 1 < count {
                    self.selected_cache_index += 1;
                } else {
                    self.selected_cache_index = 0;
                }
                self.cache_table_state.select(Some(self.selected_cache_index));
            }
        }
    }

    pub fn page_up(&mut self, step: usize) {
        match self.active_tab {
            ActiveTab::Projects => {
                self.selected_table_index = self.selected_table_index.saturating_sub(step);
                self.table_state.select(Some(self.selected_table_index));
            }
            ActiveTab::GlobalCaches => {
                self.selected_cache_index = self.selected_cache_index.saturating_sub(step);
                self.cache_table_state.select(Some(self.selected_cache_index));
            }
        }
    }

    pub fn page_down(&mut self, step: usize) {
        match self.active_tab {
            ActiveTab::Projects => {
                let count = self.get_visible_table_items().len();
                if count > 0 {
                    self.selected_table_index = (self.selected_table_index + step).min(count - 1);
                    self.table_state.select(Some(self.selected_table_index));
                }
            }
            ActiveTab::GlobalCaches => {
                let count = self.get_visible_global_caches().len();
                if count > 0 {
                    self.selected_cache_index = (self.selected_cache_index + step).min(count - 1);
                    self.cache_table_state.select(Some(self.selected_cache_index));
                }
            }
        }
    }

    pub fn move_to_top(&mut self) {
        match self.active_tab {
            ActiveTab::Projects => {
                self.selected_table_index = 0;
                self.table_state.select(Some(0));
            }
            ActiveTab::GlobalCaches => {
                self.selected_cache_index = 0;
                self.cache_table_state.select(Some(0));
            }
        }
    }

    pub fn move_to_bottom(&mut self) {
        match self.active_tab {
            ActiveTab::Projects => {
                let count = self.get_visible_table_items().len();
                if count > 0 {
                    self.selected_table_index = count - 1;
                    self.table_state.select(Some(self.selected_table_index));
                }
            }
            ActiveTab::GlobalCaches => {
                let count = self.get_visible_global_caches().len();
                if count > 0 {
                    self.selected_cache_index = count - 1;
                    self.cache_table_state.select(Some(self.selected_cache_index));
                }
            }
        }
    }

    pub fn toggle_selection(&mut self) {
        match self.active_tab {
            ActiveTab::Projects => {
                let visible = self.get_visible_table_items();
                if let Some(target) = visible.get(self.selected_table_index) {
                    match target {
                        TableItem::GroupHeader { group_key, .. } => {
                            let key = group_key.clone();
                            let group_arts: Vec<usize> = self
                                .artifacts
                                .iter()
                                .enumerate()
                                .filter(|(_, a)| {
                                    let art_key = if a.root_project_name.is_empty() {
                                        &a.display_path
                                    } else {
                                        &a.root_project_name
                                    };
                                    art_key == &key && !a.is_deleted
                                })
                                .map(|(idx, _)| idx)
                                .collect();

                            let any_unselected = group_arts.iter().any(|&i| !self.artifacts[i].is_selected);
                            for idx in group_arts {
                                self.artifacts[idx].is_selected = any_unselected;
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
            ActiveTab::GlobalCaches => {
                let visible = self.get_visible_global_caches();
                if let Some(&(original_idx, _)) = visible.get(self.selected_cache_index) {
                    if let Some(cache) = self.global_caches.get_mut(original_idx) {
                        if !cache.is_deleted {
                            cache.is_selected = !cache.is_selected;
                        }
                    }
                }
            }
        }
    }

    pub fn toggle_expand(&mut self) {
        if self.active_tab != ActiveTab::Projects {
            return;
        }
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
                TableItem::ChildArtifact { .. } => {}
            }
        }
    }

    pub fn expand_group(&mut self) {
        if self.active_tab != ActiveTab::Projects {
            return;
        }
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
        if self.active_tab != ActiveTab::Projects {
            return;
        }
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
                        self.table_state.select(Some(header_idx));
                    }
                }
            }
        }
    }

    pub fn toggle_expand_all(&mut self) {
        if self.active_tab != ActiveTab::Projects {
            return;
        }
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
        match self.active_tab {
            ActiveTab::Projects => {
                let any_selected = self.artifacts.iter().any(|a| a.is_selected && !a.is_deleted);
                for art in &mut self.artifacts {
                    if !art.is_deleted {
                        art.is_selected = !any_selected;
                    }
                }
            }
            ActiveTab::GlobalCaches => {
                let (indices, any_selected) = {
                    let visible = self.get_visible_global_caches();
                    let any_selected = visible.iter().any(|(_, c)| c.is_selected && !c.is_deleted);
                    let indices: Vec<usize> = visible.into_iter().map(|(idx, _)| idx).collect();
                    (indices, any_selected)
                };
                for original_idx in indices {
                    if let Some(cache) = self.global_caches.get_mut(original_idx) {
                        if !cache.is_deleted {
                            cache.is_selected = !any_selected;
                        }
                    }
                }
            }
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
    }

    pub fn perform_deletion(&mut self, mode: DeleteMode) {
        if let Some(ref cancel) = self.deletion_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.deletion_cancel = Some(cancel.clone());

        let (tx, rx) = crossbeam_channel::unbounded();
        self.deletion_rx = Some(rx);

        let targets: Vec<(PathBuf, u64)> = match self.active_tab {
            ActiveTab::Projects => {
                let mut list = Vec::new();
                for art in &mut self.artifacts {
                    if art.is_selected && !art.is_deleted {
                        self.deleting_paths.insert(art.target_path.clone());
                        list.push((art.target_path.clone(), art.size_bytes));
                    }
                }
                list
            }
            ActiveTab::GlobalCaches => {
                let mut list = Vec::new();
                for cache in &mut self.global_caches {
                    if cache.is_selected && !cache.is_deleted {
                        self.deleting_paths.insert(cache.path.clone());
                        list.push((cache.path.clone(), cache.size_bytes));
                    }
                }
                list
            }
        };

        let total_targets = targets.len();
        let total_bytes: u64 = targets.iter().map(|(_, size)| *size).sum();
        let initial_path = targets
            .first()
            .map(|(p, _)| p.display().to_string())
            .unwrap_or_default();

        self.deletion_state = DeletionState::Deleting {
            current_target: if total_targets > 0 { 1 } else { 0 },
            total_targets,
            completed_targets: 0,
            current_path: initial_path,
            freed_bytes: 0,
            total_bytes,
            mode,
        };

        let cancel_thread = cancel.clone();
        thread::spawn(move || {
            let mut total_freed_bytes = 0u64;
            let mut errors = 0usize;
            let mut target_errors: Vec<DeletionTargetError> = Vec::new();

            for (idx, (path, target_size)) in targets.into_iter().enumerate() {
                if cancel_thread.load(Ordering::Relaxed) {
                    break;
                }
                let path_display = path.display().to_string();
                let current_target = idx + 1;
                let completed_targets = idx;

                let _ = tx.send(DeleteProgressMessage::Progress {
                    current_target,
                    total_targets,
                    completed_targets,
                    current_path: path_display.clone(),
                    freed_bytes: total_freed_bytes,
                    total_bytes,
                    mode,
                });

                let tx_clone = tx.clone();
                let path_disp_clone = path_display.clone();

                let current_target_freed = Arc::new(AtomicU64::new(0));
                let target_freed_clone = current_target_freed.clone();

                let result = delete_path_with_progress(
                    &path,
                    mode,
                    &cancel_thread,
                    move |incremental_bytes| {
                        let current = target_freed_clone.fetch_add(incremental_bytes, Ordering::Relaxed) + incremental_bytes;
                        let _ = tx_clone.send(DeleteProgressMessage::Progress {
                            current_target,
                            total_targets,
                            completed_targets,
                            current_path: path_disp_clone.clone(),
                            freed_bytes: total_freed_bytes + current,
                            total_bytes,
                            mode,
                        });
                    },
                );

                let freed_in_target = current_target_freed.load(Ordering::Relaxed);
                let is_cancelled = cancel_thread.load(Ordering::Relaxed);
                let success = result.is_ok();
                let target_freed = if success {
                    if freed_in_target == 0 {
                        target_size
                    } else {
                        freed_in_target
                    }
                } else {
                    freed_in_target
                };

                total_freed_bytes += target_freed;

                if !success && !is_cancelled {
                    errors += 1;
                    if let Err(e) = result {
                        target_errors.push(e);
                    }
                }

                let _ = tx.send(DeleteProgressMessage::TargetFinished {
                    path: path.clone(),
                    success,
                    freed_bytes: target_freed,
                });

                if is_cancelled {
                    break;
                }

                let _ = tx.send(DeleteProgressMessage::Progress {
                    current_target,
                    total_targets,
                    completed_targets: idx + 1,
                    current_path: path_display,
                    freed_bytes: total_freed_bytes,
                    total_bytes,
                    mode,
                });
            }

            let was_cancelled = cancel_thread.load(Ordering::Relaxed);

            let log_path = if !target_errors.is_empty() {
                log_deletion_errors(&target_errors);
                Some(get_error_log_path())
            } else {
                None
            };

            let mut error_details = Vec::new();
            for err in &target_errors {
                if err.file_errors.is_empty() {
                    error_details.push(format!("{}: {}", err.target_path.display(), err.message));
                } else {
                    for (file_p, file_msg) in &err.file_errors {
                        let display_p = file_p
                            .strip_prefix(&err.target_path)
                            .map(|rel| rel.display().to_string())
                            .unwrap_or_else(|_| file_p.display().to_string());
                        error_details.push(format!("{}: {}", display_p, file_msg));
                    }
                }
            }

            let _ = tx.send(DeleteProgressMessage::Done {
                freed_bytes: total_freed_bytes,
                errors,
                mode,
                cancelled: was_cancelled,
                error_details,
                log_path,
            });
        });
    }
}

/// Evaluates if an artifact satisfies the given search query tokens.
pub fn matches_artifact_query(a: &DiscoveredArtifact, q: &str) -> bool {
    let q = q.trim();
    if q.is_empty() {
        return true;
    }

    let tokens: Vec<&str> = q.split_whitespace().collect();
    for token in tokens {
        let lower_token = token.to_lowercase();
        if let Some(eco_prefix) = lower_token
            .strip_prefix("eco:")
            .or_else(|| lower_token.strip_prefix("ecosystem:"))
        {
            if let Some(target_eco) = Ecosystem::parse(eco_prefix) {
                if a.ecosystem != target_eco {
                    return false;
                }
            } else if !a.ecosystem.name().to_lowercase().contains(eco_prefix)
                && !a.ecosystem.badge().to_lowercase().contains(eco_prefix)
            {
                return false;
            }
        } else if let Some(size_spec) = lower_token.strip_prefix("size:>") {
            if let Some(threshold) = parse_size_str(size_spec) {
                if a.size_bytes < threshold {
                    return false;
                }
            }
        } else if let Some(size_spec) = lower_token.strip_prefix("size:<") {
            if let Some(threshold) = parse_size_str(size_spec) {
                if a.size_bytes > threshold {
                    return false;
                }
            }
        } else if lower_token == "locked:yes" || lower_token == "locked:true" {
            if !a.has_lockfile {
                return false;
            }
        } else if lower_token == "locked:no" || lower_token == "locked:false" {
            if a.has_lockfile {
                return false;
            }
        } else if lower_token == "git:clean" {
            if !a.git_clean {
                return false;
            }
        } else if lower_token == "git:dirty" {
            if a.git_clean {
                return false;
            }
        } else {
            let matches = a.root_project_name.to_lowercase().contains(&lower_token)
                || a.project_name.to_lowercase().contains(&lower_token)
                || a.display_path.to_lowercase().contains(&lower_token)
                || a.folder_name.to_lowercase().contains(&lower_token)
                || a.rule_label.to_lowercase().contains(&lower_token)
                || a.ecosystem.name().to_lowercase().contains(&lower_token)
                || a.ecosystem.badge().to_lowercase().contains(&lower_token)
                || a.target_path.to_string_lossy().to_lowercase().contains(&lower_token);
            if !matches {
                return false;
            }
        }
    }
    true
}

/// Evaluates if a global cache target satisfies the given search query tokens.
pub fn matches_global_cache_query(c: &GlobalCacheTarget, q: &str) -> bool {
    let q = q.trim();
    if q.is_empty() {
        return true;
    }

    let tokens: Vec<&str> = q.split_whitespace().collect();
    for token in tokens {
        let lower_token = token.to_lowercase();
        if let Some(eco_prefix) = lower_token
            .strip_prefix("eco:")
            .or_else(|| lower_token.strip_prefix("ecosystem:"))
        {
            if let Some(target_eco) = Ecosystem::parse(eco_prefix) {
                if c.ecosystem != target_eco {
                    return false;
                }
            } else if !c.ecosystem.name().to_lowercase().contains(eco_prefix)
                && !c.ecosystem.badge().to_lowercase().contains(eco_prefix)
            {
                return false;
            }
        } else if let Some(size_spec) = lower_token.strip_prefix("size:>") {
            if let Some(threshold) = parse_size_str(size_spec) {
                if c.size_bytes < threshold {
                    return false;
                }
            }
        } else if let Some(size_spec) = lower_token.strip_prefix("size:<") {
            if let Some(threshold) = parse_size_str(size_spec) {
                if c.size_bytes > threshold {
                    return false;
                }
            }
        } else {
            let matches = c.name.to_lowercase().contains(&lower_token)
                || c.ecosystem.name().to_lowercase().contains(&lower_token)
                || c.ecosystem.badge().to_lowercase().contains(&lower_token)
                || c.path.to_string_lossy().to_lowercase().contains(&lower_token)
                || c.description.to_lowercase().contains(&lower_token)
                || c.clean_hint.to_lowercase().contains(&lower_token);
            if !matches {
                return false;
            }
        }
    }
    true
}
