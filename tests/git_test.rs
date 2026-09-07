use std::fs;
use std::process::Command;
use degunk::core::git::{find_git_root, inspect_project_activity, GitStatus};

fn setup_git_repo(path: &std::path::Path) -> bool {
    let init = Command::new("git").args(["init"]).current_dir(path).output();
    if init.is_err() || !init.unwrap().status.success() {
        return false;
    }
    let _ = Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output();
    true
}

#[test]
fn test_inspect_git_clean_status() {
    let repo_dir = std::env::temp_dir().join("degunk_test_git_clean");
    let _ = fs::remove_dir_all(&repo_dir);
    fs::create_dir_all(&repo_dir).unwrap();

    if !setup_git_repo(&repo_dir) {
        let _ = fs::remove_dir_all(&repo_dir);
        return;
    }

    fs::write(repo_dir.join("main.rs"), b"fn main() {}").unwrap();
    let _ = Command::new("git").args(["add", "."]).current_dir(&repo_dir).output();
    let _ = Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_dir)
        .output();

    let root = find_git_root(&repo_dir);
    assert_eq!(root, Some(repo_dir.clone()));

    let activity = inspect_project_activity(&repo_dir);
    assert!(activity.is_git);
    assert_eq!(activity.git_status, GitStatus::Clean);
    assert_eq!(activity.last_commit_message.as_deref(), Some("Initial commit"));
    assert_eq!(activity.days_inactive, 0);

    let _ = fs::remove_dir_all(&repo_dir);
}

#[test]
fn test_inspect_git_dirty_status() {
    let repo_dir = std::env::temp_dir().join("degunk_test_git_dirty");
    let _ = fs::remove_dir_all(&repo_dir);
    fs::create_dir_all(&repo_dir).unwrap();

    if !setup_git_repo(&repo_dir) {
        let _ = fs::remove_dir_all(&repo_dir);
        return;
    }

    fs::write(repo_dir.join("README.md"), b"# Project").unwrap();
    let _ = Command::new("git").args(["add", "."]).current_dir(&repo_dir).output();
    let _ = Command::new("git")
        .args(["commit", "-m", "First commit"])
        .current_dir(&repo_dir)
        .output();

    // Create an untracked file to dirty the status
    fs::write(repo_dir.join("untracked.txt"), b"untracked content").unwrap();

    let activity = inspect_project_activity(&repo_dir);
    assert!(activity.is_git);
    assert_eq!(activity.git_status, GitStatus::Dirty(1));

    let _ = fs::remove_dir_all(&repo_dir);
}

#[test]
fn test_inspect_non_git_directory() {
    let plain_dir = std::env::temp_dir().join("degunk_test_non_git");
    let _ = fs::remove_dir_all(&plain_dir);
    fs::create_dir_all(&plain_dir).unwrap();

    fs::write(plain_dir.join("notes.txt"), b"just regular files").unwrap();

    let root = find_git_root(&plain_dir);
    assert_eq!(root, None);

    let activity = inspect_project_activity(&plain_dir);
    assert!(!activity.is_git);
    assert_eq!(activity.git_status, GitStatus::NotGit);
    assert!(activity.last_commit_message.is_none());

    let _ = fs::remove_dir_all(&plain_dir);
}
