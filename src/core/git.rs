use chrono::{DateTime, Local, TimeZone, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

static GIT_AVAILABLE: OnceLock<bool> = OnceLock::new();
static GIT_REPO_CACHE: OnceLock<RwLock<HashMap<PathBuf, (usize, usize)>>> = OnceLock::new();
static GIT_REPO_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();

fn is_git_available() -> bool {
    *GIT_AVAILABLE.get_or_init(|| {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    })
}

fn get_repo_cache() -> &'static RwLock<HashMap<PathBuf, (usize, usize)>> {
    GIT_REPO_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_repo_lock(git_root: &Path) -> Arc<Mutex<()>> {
    let locks = GIT_REPO_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = locks.lock().unwrap();
    guard
        .entry(git_root.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitStatus {
    Clean,
    Dirty(usize),       // Count of changed / untracked files
    Unpushed(usize),    // Count of unpushed commits
    DirtyAndUnpushed { changed: usize, unpushed: usize },
    NotGit,
}

#[derive(Debug, Clone)]
pub struct ProjectActivity {
    pub is_git: bool,
    pub git_status: GitStatus,
    pub last_active: DateTime<Local>,
    pub days_inactive: u32,
    pub last_commit_message: Option<String>,
}

/// Finds the root of the git repository if the path is inside one.
pub fn find_git_root(start_path: &Path) -> Option<PathBuf> {
    let mut current = start_path.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Inspects the git and filesystem activity of a project directory.
pub fn inspect_project_activity(project_dir: &Path) -> ProjectActivity {
    if is_git_available() {
        if let Some(git_root) = find_git_root(project_dir) {
            if let Some(activity) = inspect_git_repo(&git_root, project_dir) {
                return activity;
            }
        }
    }

    // Fallback to filesystem metadata
    let modified_time = get_last_modified_time(project_dir);
    let now = Local::now();
    let duration = now.signed_duration_since(modified_time);
    let days_inactive = duration.num_days().max(0) as u32;

    ProjectActivity {
        is_git: false,
        git_status: GitStatus::NotGit,
        last_active: modified_time,
        days_inactive,
        last_commit_message: None,
    }
}

fn inspect_git_repo(git_root: &Path, project_dir: &Path) -> Option<ProjectActivity> {
    let rel_path = project_dir.strip_prefix(git_root).unwrap_or(Path::new(""));
    let rel_str = rel_path.to_string_lossy();

    // 1. Get last commit timestamp and message for this specific subproject
    let mut log_cmd = Command::new("git");
    log_cmd.args(["log", "-1", "--format=%ct%x00%s"]);
    if !rel_str.is_empty() {
        log_cmd.args(["--", rel_str.as_ref()]);
    }
    let mut log_output = log_cmd.current_dir(git_root).output().ok();

    // If subpath log failed or empty, fallback to repository-level git log
    if let Some(ref out) = log_output {
        if !out.status.success() || out.stdout.is_empty() {
            log_output = Command::new("git")
                .args(["log", "-1", "--format=%ct%x00%s"])
                .current_dir(git_root)
                .output()
                .ok();
        }
    }

    let (last_active, commit_msg) = if let Some(ref out) = log_output {
        if out.status.success() && !out.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut parts = stdout.trim().splitn(2, '\0');
            let timestamp_str = parts.next().unwrap_or("");
            let msg = parts.next().map(|s| s.to_string());

            if let Ok(ts) = timestamp_str.parse::<i64>() {
                if let Some(dt_utc) = Utc.timestamp_opt(ts, 0).single() {
                    (DateTime::<Local>::from(dt_utc), msg)
                } else {
                    (get_last_modified_time(project_dir), None)
                }
            } else {
                (get_last_modified_time(project_dir), None)
            }
        } else {
            (get_last_modified_time(project_dir), None)
        }
    } else {
        (get_last_modified_time(project_dir), None)
    };

    let now = Local::now();
    let duration = now.signed_duration_since(last_active);
    let days_inactive = duration.num_days().max(0) as u32;

    // 2. Query or retrieve cached repo status with per-repository mutual exclusion
    let (repo_changed_count, unpushed_count) = {
        let cached = {
            let cache_read = get_repo_cache().read().unwrap();
            cache_read.get(git_root).copied()
        };

        if let Some(counts) = cached {
            counts
        } else {
            let repo_lock = get_repo_lock(git_root);
            let _guard = repo_lock.lock().unwrap();

            let cached_again = {
                let cache_read = get_repo_cache().read().unwrap();
                cache_read.get(git_root).copied()
            };

            if let Some(counts) = cached_again {
                counts
            } else {
                let status_out = Command::new("git")
                    .args(["status", "--porcelain"])
                    .current_dir(git_root)
                    .output()
                    .ok();
                let changed = match status_out {
                    Some(ref out) if out.status.success() => {
                        String::from_utf8_lossy(&out.stdout)
                            .lines()
                            .filter(|l| !l.trim().is_empty())
                            .count()
                    }
                    _ => 0,
                };

                let ahead_out = Command::new("git")
                    .args(["rev-list", "--count", "@{u}..HEAD"])
                    .current_dir(git_root)
                    .output()
                    .ok();
                let unpushed = match ahead_out {
                    Some(ref out) if out.status.success() => {
                        String::from_utf8_lossy(&out.stdout)
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(0)
                    }
                    _ => 0,
                };

                let mut cache_write = get_repo_cache().write().unwrap();
                cache_write.insert(git_root.to_path_buf(), (changed, unpushed));
                (changed, unpushed)
            }
        }
    };

    // If repo is clean or this is the root project, use repo-level count directly
    let changed_count = if repo_changed_count == 0 || rel_str.is_empty() {
        repo_changed_count
    } else {
        // Scoped check only when the repo has modifications.
        // Acquire repo_lock to avoid index.lock contention during concurrent subproject checks.
        let repo_lock = get_repo_lock(git_root);
        let _guard = repo_lock.lock().unwrap();

        let status_output = Command::new("git")
            .args(["status", "--porcelain", "--", rel_str.as_ref()])
            .current_dir(git_root)
            .output()
            .ok();
        match status_output {
            Some(ref out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .count()
            }
            _ => 0,
        }
    };

    let git_status = match (changed_count > 0, unpushed_count > 0) {
        (true, true) => GitStatus::DirtyAndUnpushed {
            changed: changed_count,
            unpushed: unpushed_count,
        },
        (true, false) => GitStatus::Dirty(changed_count),
        (false, true) => GitStatus::Unpushed(unpushed_count),
        (false, false) => GitStatus::Clean,
    };

    Some(ProjectActivity {
        is_git: true,
        git_status,
        last_active,
        days_inactive,
        last_commit_message: commit_msg,
    })
}

fn get_last_modified_time(path: &Path) -> DateTime<Local> {
    if let Ok(metadata) = std::fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            let dt: DateTime<Local> = DateTime::from(modified);
            return dt;
        }
    }
    Local::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;

    #[test]
    fn test_concurrent_monorepo_git_inspection() {
        let current_dir = std::env::current_dir().unwrap();
        if find_git_root(&current_dir).is_none() {
            return;
        }

        let subdirs = vec![
            current_dir.join("src"),
            current_dir.join("src/core"),
            current_dir.join("src/ui"),
            current_dir.join("tests"),
        ];

        let results: Vec<ProjectActivity> = (0..32)
            .into_par_iter()
            .map(|i| {
                let path = &subdirs[i % subdirs.len()];
                inspect_project_activity(path)
            })
            .collect();

        assert_eq!(results.len(), 32);
        for res in results {
            assert!(res.is_git);
        }
    }
}
