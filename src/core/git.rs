use chrono::{DateTime, Local, TimeZone, Utc};
use std::path::{Path, PathBuf};
use std::process::Command;

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
    if let Some(git_root) = find_git_root(project_dir) {
        if let Some(activity) = inspect_git_repo(&git_root) {
            return activity;
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

fn inspect_git_repo(git_root: &Path) -> Option<ProjectActivity> {
    // 1. Get last commit timestamp and message: git log -1 --format=%ct%x00%s
    let log_output = Command::new("git")
        .args(["log", "-1", "--format=%ct\0%s"])
        .current_dir(git_root)
        .output()
        .ok();

    let (last_active, commit_msg) = if let Some(ref out) = log_output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut parts = stdout.trim().splitn(2, '\0');
            let timestamp_str = parts.next().unwrap_or("");
            let msg = parts.next().map(|s| s.to_string());

            if let Ok(ts) = timestamp_str.parse::<i64>() {
                if let Some(dt_utc) = Utc.timestamp_opt(ts, 0).single() {
                    (DateTime::<Local>::from(dt_utc), msg)
                } else {
                    (get_last_modified_time(git_root), None)
                }
            } else {
                (get_last_modified_time(git_root), None)
            }
        } else {
            (get_last_modified_time(git_root), None)
        }
    } else {
        (get_last_modified_time(git_root), None)
    };

    let now = Local::now();
    let duration = now.signed_duration_since(last_active);
    let days_inactive = duration.num_days().max(0) as u32;

    // 2. Check git status: git status --porcelain
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(git_root)
        .output()
        .ok();

    let changed_count = match status_output {
        Some(ref out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count()
        }
        _ => 0,
    };

    // 3. Check ahead count: git rev-list --count @{u}..HEAD
    let ahead_output = Command::new("git")
        .args(["rev-list", "--count", "@{u}..HEAD"])
        .current_dir(git_root)
        .output()
        .ok();

    let unpushed_count = match ahead_output {
        Some(ref out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<usize>()
                .unwrap_or(0)
        }
        _ => 0,
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
