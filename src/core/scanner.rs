use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use crate::core::ecosystem::{has_lockfile, match_rule, Ecosystem};
use crate::core::git::{inspect_project_activity, ProjectActivity};
use crate::core::size::calculate_dir_size;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredArtifact {
    pub id: usize,
    pub target_path: PathBuf,
    pub project_path: PathBuf,
    pub project_name: String,
    pub folder_name: String,
    pub ecosystem: Ecosystem,
    pub rule_label: String,
    pub reinstall_cmd: String,
    pub size_bytes: u64,
    pub file_count: usize,
    pub size_calculated: bool,
    pub has_lockfile: bool,
    pub lockfile_name: Option<String>,
    #[serde(skip)]
    pub activity: Option<ProjectActivity>,
    pub days_inactive: u32,
    pub is_git: bool,
    pub git_clean: bool,
    pub is_selected: bool,
    pub is_deleted: bool,
}

#[derive(Debug, Clone)]
pub enum ScanMessage {
    Found(DiscoveredArtifact),
    SizeUpdated {
        id: usize,
        size_bytes: u64,
        file_count: usize,
    },
    ActivityUpdated {
        id: usize,
        activity: ProjectActivity,
    },
    Progress {
        scanned_dirs: usize,
    },
    Finished {
        total_scanned_dirs: usize,
    },
}

/// System and hidden directories that should never be searched
fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | "$Recycle.Bin"
            | "$RECYCLE.BIN"
            | ".Trash"
            | ".Trash-1000"
            | "System Volume Information"
            | "Windows"
            | "Program Files"
            | "Program Files (x86)"
            | "AppData"
            | "Library"
    )
}

pub struct Scanner {
    cancel_flag: Arc<AtomicBool>,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        self.cancel_flag.clone()
    }

    /// Spawns background threads to scan roots and stream results to the sender.
    pub fn start_scan(
        roots: Vec<PathBuf>,
        tx: Sender<ScanMessage>,
        cancel_flag: Arc<AtomicBool>,
        allowed_ecosystems: Option<HashSet<Ecosystem>>,
    ) {
        thread::spawn(move || {
            let id_counter = Arc::new(AtomicUsize::new(0));
            let total_dirs = Arc::new(AtomicUsize::new(0));
            let pending_tasks = Arc::new(AtomicUsize::new(0));

            for root in roots {
                if cancel_flag.load(Ordering::Relaxed) {
                    break;
                }
                scan_directory_recursive(
                    &root,
                    &tx,
                    &cancel_flag,
                    &id_counter,
                    &total_dirs,
                    &pending_tasks,
                    &allowed_ecosystems,
                );
            }

            // Wait for all spawned background size calculations to complete
            while pending_tasks.load(Ordering::SeqCst) > 0 {
                if cancel_flag.load(Ordering::Relaxed) {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(5));
            }

            let _ = tx.send(ScanMessage::Finished {
                total_scanned_dirs: total_dirs.load(Ordering::Relaxed),
            });
        });
    }
}

fn scan_directory_recursive(
    current_dir: &Path,
    tx: &Sender<ScanMessage>,
    cancel_flag: &Arc<AtomicBool>,
    id_counter: &Arc<AtomicUsize>,
    total_dirs: &Arc<AtomicUsize>,
    pending_tasks: &Arc<AtomicUsize>,
    allowed_ecosystems: &Option<HashSet<Ecosystem>>,
) {
    if cancel_flag.load(Ordering::Relaxed) {
        return;
    }

    let read_res = match std::fs::read_dir(current_dir) {
        Ok(res) => res,
        Err(_) => return,
    };

    total_dirs.fetch_add(1, Ordering::Relaxed);

    // Periodically send progress
    let count = total_dirs.load(Ordering::Relaxed);
    if count % 100 == 0 {
        let _ = tx.send(ScanMessage::Progress {
            scanned_dirs: count,
        });
    }

    let mut subdirs_to_recurse = Vec::new();

    for entry in read_res.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if !file_type.is_dir() {
            continue;
        }

        let folder_name = entry.file_name().to_string_lossy().to_string();

        if should_skip_dir(&folder_name) {
            continue;
        }

        let target_path = entry.path();

        // Check if this folder is an artifact of the current directory
        if let Some(rule) = match_rule(&folder_name, current_dir) {
            if let Some(ref allowed) = allowed_ecosystems {
                if !allowed.contains(&rule.ecosystem) {
                    continue;
                }
            }

            let id = id_counter.fetch_add(1, Ordering::Relaxed);
            let (has_lock, lock_name) = has_lockfile(current_dir, rule.lockfiles);

            let project_name = current_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| current_dir.to_string_lossy().to_string());

            let artifact = DiscoveredArtifact {
                id,
                target_path: target_path.clone(),
                project_path: current_dir.to_path_buf(),
                project_name,
                folder_name: folder_name.clone(),
                ecosystem: rule.ecosystem,
                rule_label: rule.label.to_string(),
                reinstall_cmd: rule.reinstall_cmd.to_string(),
                size_bytes: 0,
                file_count: 0,
                size_calculated: false,
                has_lockfile: has_lock,
                lockfile_name: lock_name,
                activity: None,
                days_inactive: 0,
                is_git: false,
                git_clean: true,
                is_selected: false,
                is_deleted: false,
            };

            // Emit discovery immediately
            let _ = tx.send(ScanMessage::Found(artifact));

            // Spawn background task to compute size and inspect git/activity
            pending_tasks.fetch_add(1, Ordering::SeqCst);
            let pending_clone = pending_tasks.clone();
            let tx_clone = tx.clone();
            let target_path_clone = target_path.clone();
            let project_path_clone = current_dir.to_path_buf();

            rayon::spawn(move || {
                // Compute size
                let stats = calculate_dir_size(&target_path_clone);
                let _ = tx_clone.send(ScanMessage::SizeUpdated {
                    id,
                    size_bytes: stats.bytes,
                    file_count: stats.file_count,
                });

                // Inspect activity
                let activity = inspect_project_activity(&project_path_clone);
                let _ = tx_clone.send(ScanMessage::ActivityUpdated { id, activity });

                pending_clone.fetch_sub(1, Ordering::SeqCst);
            });

            // Prune: Do NOT recurse into artifact directories (e.g. node_modules)
            continue;
        }

        subdirs_to_recurse.push(target_path);
    }

    // Recurse into non-artifact subdirectories
    for subdir in subdirs_to_recurse {
        scan_directory_recursive(
            &subdir,
            tx,
            cancel_flag,
            id_counter,
            total_dirs,
            pending_tasks,
            allowed_ecosystems,
        );
    }
}
