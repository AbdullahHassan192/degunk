use crossbeam_channel::Receiver;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::core::deleter::{delete_path_with_progress, DeleteMode, DeleteProgressMessage};
use crate::core::ecosystem::Ecosystem;
use crate::core::global_cache::{
    detect_global_caches, start_global_cache_scan, GlobalCacheMessage, GlobalCacheTarget,
};
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
    pub deleting_paths: HashSet<PathBuf>,
    pub should_quit: bool,
    pub collapsed_groups: HashSet<String>,

    pub active_tab: ActiveTab,
    pub global_caches: Vec<GlobalCacheTarget>,
    pub selected_cache_index: usize,

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
            deleting_paths: HashSet::new(),
            should_quit: false,
            collapsed_groups: HashSet::new(),
            active_tab: ActiveTab::Projects,
            global_caches: Vec::new(),
            selected_cache_index: 0,
            scanner_cancel: None,
            rx: None,
            global_cache_cancel: None,
            global_cache_rx: None,
            deletion_rx: None,
        };
        app.start_scan();
        app
    }

    pub fn start_scan(&mut self) {
        if let Some(ref cancel) = self.scanner_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(ref cancel) = self.global_cache_cancel {
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

        // Scan global tool caches in background
        self.global_caches = detect_global_caches();
        let (gc_tx, gc_rx) = crossbeam_channel::unbounded();
        let gc_cancel = Arc::new(AtomicBool::new(false));
        self.global_cache_cancel = Some(gc_cancel.clone());
        self.global_cache_rx = Some(gc_rx);
        start_global_cache_scan(self.global_caches.clone(), gc_tx, gc_cancel);

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
                    DeleteProgressMessage::TargetFinished { path, success } => {
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
                        }
                    }
                    DeleteProgressMessage::Done {
                        freed_bytes,
                        errors,
                        mode,
                    } => {
                        self.deleting_paths.clear();
                        self.deletion_state = DeletionState::Done {
                            freed_bytes,
                            errors,
                            mode,
                        };
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

        thread::spawn(move || {
            let mut total_freed_bytes = 0u64;
            let mut errors = 0usize;

            for (idx, (path, target_size)) in targets.into_iter().enumerate() {
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

                let mut current_target_freed = 0u64;
                let result = delete_path_with_progress(
                    &path,
                    mode,
                    move |incremental_bytes| {
                        current_target_freed += incremental_bytes;
                        let _ = tx_clone.send(DeleteProgressMessage::Progress {
                            current_target,
                            total_targets,
                            completed_targets,
                            current_path: path_disp_clone.clone(),
                            freed_bytes: total_freed_bytes + current_target_freed,
                            total_bytes,
                            mode,
                        });
                    },
                );

                let success = result.is_ok();
                if success {
                    if current_target_freed == 0 {
                        total_freed_bytes += target_size;
                    } else {
                        total_freed_bytes += current_target_freed;
                    }
                } else {
                    errors += 1;
                }

                let _ = tx.send(DeleteProgressMessage::TargetFinished {
                    path: path.clone(),
                    success,
                });

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

            let _ = tx.send(DeleteProgressMessage::Done {
                freed_bytes: total_freed_bytes,
                errors,
                mode,
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
