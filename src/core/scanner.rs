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
    pub display_path: String,
    pub root_project_path: PathBuf,
    pub root_project_name: String,
    pub sub_path: String,
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

/// System, tool-cache, SDK, and cloud storage directories that should never be searched by default
fn should_skip_dir(name: &str, path: &Path, include_cloud: bool) -> bool {
    let lower_name = name.to_lowercase();

    // 1. Cloud storage protection (unless explicitly included)
    if !include_cloud {
        if lower_name.starts_with("onedrive")
            || lower_name.starts_with("google drive")
            || lower_name.starts_with("dropbox")
            || lower_name.starts_with("icloud")
            || lower_name.starts_with("box sync")
            || lower_name.starts_with("creative cloud")
            || lower_name == ".dropbox.cache"
        {
            return true;
        }

        // Check against system OneDrive environment paths
        for var_name in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
            if let Ok(val) = std::env::var(var_name) {
                let od_path = PathBuf::from(val);
                if path.starts_with(&od_path) {
                    return true;
                }
            }
        }
    }

    // 2. OS & Version Control internals
    if matches!(
        lower_name.as_str(),
        ".git"
            | ".hg"
            | ".svn"
            | "$recycle.bin"
            | ".trash"
            | ".trash-1000"
            | "system volume information"
            | "windows"
            | "program files"
            | "program files (x86)"
            | "appdata"
            | "library"
            | "perflogs"
    ) {
        return true;
    }

    // 3. Tool runtimes, package manager caches, IDE extensions, and SDK internals
    if matches!(
        lower_name.as_str(),
        ".vscode-test"
            | ".vscode"
            | ".vscode-shared"
            | ".antigravity"
            | ".antigravity-ide"
            | ".cursor"
            | ".claude"
            | ".codex"
            | ".gemini"
            | ".eclipse"
            | ".p2"
            | ".redhat"
            | ".android"
            | "flutter sdk"
            | "flutter_sdk"
            | ".flutter-sdk"
            | ".pub-cache"
            | ".cargo"
            | ".rustup"
            | ".m2"
            | ".conda"
            | ".bun"
            | "anaconda3"
            | "miniconda3"
            | ".nuget"
            | "site-packages"
            | ".cache"
    ) {
        return true;
    }

    false
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
        include_cloud: bool,
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
                    &root,
                    &tx,
                    &cancel_flag,
                    &id_counter,
                    &total_dirs,
                    &pending_tasks,
                    &allowed_ecosystems,
                    include_cloud,
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
    root_dir: &Path,
    tx: &Sender<ScanMessage>,
    cancel_flag: &Arc<AtomicBool>,
    id_counter: &Arc<AtomicUsize>,
    total_dirs: &Arc<AtomicUsize>,
    pending_tasks: &Arc<AtomicUsize>,
    allowed_ecosystems: &Option<HashSet<Ecosystem>>,
    include_cloud: bool,
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
        let target_path = entry.path();

        if should_skip_dir(&folder_name, &target_path, include_cloud) {
            continue;
        }

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

            // Compute relative display path for context (e.g. "look-busy/look-busy" or "flut/impromptu")
            let display_path = match current_dir.strip_prefix(root_dir) {
                Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().replace('\\', "/"),
                _ => project_name.clone(),
            };

            let (root_project_path, root_project_name, sub_path) = resolve_root_project(current_dir, root_dir);

            let artifact = DiscoveredArtifact {
                id,
                target_path: target_path.clone(),
                project_path: current_dir.to_path_buf(),
                project_name,
                display_path,
                root_project_path,
                root_project_name,
                sub_path,
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
            root_dir,
            tx,
            cancel_flag,
            id_counter,
            total_dirs,
            pending_tasks,
            allowed_ecosystems,
            include_cloud,
        );
    }
}

/// Determines the logical top-level project root and relative subproject path for an artifact.
pub fn resolve_root_project(current_dir: &Path, root_dir: &Path) -> (PathBuf, String, String) {
    // 1. Check if inside a git repository root that is under or equal to root_dir
    if let Some(git_root) = crate::core::git::find_git_root(current_dir) {
        if git_root.starts_with(root_dir) && git_root != root_dir {
            let root_display = match git_root.strip_prefix(root_dir) {
                Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().replace('\\', "/"),
                _ => git_root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| git_root.to_string_lossy().to_string()),
            };
            let sub_path = match current_dir.strip_prefix(&git_root) {
                Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().replace('\\', "/"),
                _ => String::new(),
            };
            return (git_root, root_display, sub_path);
        }
    }

    // 2. Walk upwards from current_dir towards root_dir to find ancestor with root project manifest
    let mut curr = current_dir.to_path_buf();
    let mut best_root = current_dir.to_path_buf();
    while curr != root_dir {
        if let Some(parent) = curr.parent() {
            if parent == root_dir {
                if best_root == current_dir {
                    best_root = curr.clone();
                }
                break;
            }
            let has_manifest = parent.join("Cargo.toml").exists()
                || parent.join("package.json").exists()
                || parent.join("pubspec.yaml").exists()
                || parent.join("pyproject.toml").exists()
                || parent.join("go.mod").exists()
                || parent.join(".git").exists();
            if has_manifest {
                best_root = parent.to_path_buf();
            }
            curr = parent.to_path_buf();
        } else {
            break;
        }
    }

    let root_display = match best_root.strip_prefix(root_dir) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().replace('\\', "/"),
        _ => best_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| best_root.to_string_lossy().to_string()),
    };

    let sub_path = match current_dir.strip_prefix(&best_root) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().replace('\\', "/"),
        _ => String::new(),
    };

    (best_root, root_display, sub_path)
}

