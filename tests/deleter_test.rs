use std::fs::{self, File};
use std::io::Write;
use std::sync::atomic::AtomicBool;

use degunk::core::deleter::{
    delete_path, delete_path_with_progress, get_error_log_path, log_deletion_errors,
    strip_readonly_recursive, DeleteMode, DeletionTargetError,
};

#[test]
fn test_delete_readonly_directory() {
    let temp_dir = std::env::temp_dir().join("degunk_test_readonly_del");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let sub_dir = temp_dir.join("nested");
    fs::create_dir_all(&sub_dir).unwrap();

    let file_path = sub_dir.join("readonly_file.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "This file is read only").unwrap();
    drop(f);

    // Mark the file as read-only
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file_path, perms).unwrap();

    // Verify it is indeed read-only
    assert!(fs::metadata(&file_path).unwrap().permissions().readonly());

    // Call delete_path in Permanent mode
    let result = delete_path(&temp_dir, DeleteMode::Permanent);
    assert!(result.is_ok(), "delete_path failed on readonly file: {:?}", result);
    assert!(!temp_dir.exists(), "Directory should have been completely deleted");
}

#[test]
fn test_strip_readonly_recursive() {
    let temp_dir = std::env::temp_dir().join("degunk_test_strip_readonly");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let sub_dir = temp_dir.join("nested_folder");
    fs::create_dir_all(&sub_dir).unwrap();

    let file_path = sub_dir.join("package-lock.json");
    fs::write(&file_path, b"{}").unwrap();

    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file_path, perms).unwrap();
    assert!(fs::metadata(&file_path).unwrap().permissions().readonly());

    let cancel = AtomicBool::new(false);
    let strip_res = strip_readonly_recursive(&temp_dir, &cancel);
    assert!(strip_res.is_ok());

    let current_perms = fs::metadata(&file_path).unwrap().permissions();
    assert!(!current_perms.readonly(), "File read-only bit should be cleared");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[cfg(windows)]
#[test]
fn test_locked_file_error_capture() {
    use std::os::windows::fs::OpenOptionsExt;

    let temp_dir = std::env::temp_dir().join("degunk_test_locked_binary");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let locked_binary = temp_dir.join("active_server.exe");
    fs::write(&locked_binary, b"MZ mock executable bytes").unwrap();

    // Lock file exclusively with share_mode(0)
    let _lock_handle = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&locked_binary)
        .unwrap();

    // Attempt deletion
    let result = delete_path(&temp_dir, DeleteMode::Permanent);
    assert!(result.is_err(), "Deletion must fail on locked file");

    let err = result.unwrap_err();
    assert_eq!(err.target_path, temp_dir);
    assert!(
        !err.file_errors.is_empty(),
        "Must record locked file in file_errors"
    );

    let (failed_path, error_msg) = &err.file_errors[0];
    assert!(failed_path.ends_with("active_server.exe"));
    assert!(
        error_msg.contains("Access is denied")
            || error_msg.contains("used by another process")
            || error_msg.contains("5")
            || error_msg.contains("32"),
        "Error message must specify OS lock failure reason, got: {}",
        error_msg
    );

    drop(_lock_handle);
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_error_logging_mechanism() {
    let err = DeletionTargetError::with_file_errors(
        std::env::temp_dir().join("mock_target"),
        "Mock directory deletion failed",
        vec![(
            std::env::temp_dir().join("mock_target").join("mock.exe"),
            "Access is denied (os error 5)".to_string(),
        )],
    );

    log_deletion_errors(&[err]);
    let log_path = get_error_log_path();
    assert!(log_path.exists(), "Error log file must be created on disk");

    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("mock.exe"));
    assert!(content.contains("Access is denied (os error 5)"));
}

#[test]
fn test_delete_path_with_progress() {
    let temp_dir = std::env::temp_dir().join("degunk_test_progress_del");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let mut expected_bytes = 0u64;
    for i in 0..5 {
        let file_path = temp_dir.join(format!("file_{}.bin", i));
        let content = vec![b'A' + i as u8; (i + 1) * 1024];
        expected_bytes += content.len() as u64;
        fs::write(&file_path, content).unwrap();
    }

    let mut freed_accum = 0u64;
    let cancel = AtomicBool::new(false);
    let result = delete_path_with_progress(
        &temp_dir,
        DeleteMode::Permanent,
        &cancel,
        |bytes| {
            freed_accum += bytes;
        },
    );

    assert!(result.is_ok(), "delete_path_with_progress failed: {:?}", result);
    assert!(!temp_dir.exists(), "Target directory must be deleted");
    assert_eq!(freed_accum, expected_bytes, "Freed bytes must match expected file size sum");
}

#[test]
fn test_delete_path_cancelled() {
    let temp_dir = std::env::temp_dir().join("degunk_test_cancel_del");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    for i in 0..5 {
        let file_path = temp_dir.join(format!("file_{}.bin", i));
        fs::write(&file_path, b"test cancellation").unwrap();
    }

    // Set cancel to true before execution
    let cancel = AtomicBool::new(true);
    let mut freed_accum = 0u64;
    let result = delete_path_with_progress(
        &temp_dir,
        DeleteMode::Permanent,
        &cancel,
        |bytes| {
            freed_accum += bytes;
        },
    );

    assert!(result.is_err(), "delete_path_with_progress should abort on cancel");
    assert_eq!(freed_accum, 0, "No bytes should be freed if cancelled beforehand");
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_delete_path_cancelled_mid_flight() {
    let temp_dir = std::env::temp_dir().join("degunk_test_cancel_mid_flight");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create 1500 files to span multiple chunk iterations (chunk size is 1000)
    for i in 0..1500 {
        let file_path = temp_dir.join(format!("file_{}.bin", i));
        fs::write(&file_path, b"abcdefghij").unwrap();
    }

    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let cancel_in_callback = cancel.clone();
    let freed_accum = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let freed_clone = freed_accum.clone();

    let result = delete_path_with_progress(
        &temp_dir,
        DeleteMode::Permanent,
        &cancel,
        move |bytes| {
            freed_clone.fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
            // Cancel after first batch
            cancel_in_callback.store(true, std::sync::atomic::Ordering::Relaxed);
        },
    );

    assert!(result.is_err(), "delete_path_with_progress must report cancelled");
    let freed = freed_accum.load(std::sync::atomic::Ordering::Relaxed);
    assert!(freed > 0, "Partial bytes must be recorded before cancellation");
    assert!(freed < 1500 * 10, "Not all files should have been deleted");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_delete_single_file_permanent() {
    let temp_dir = std::env::temp_dir().join("degunk_test_single_file");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("isolated_file.txt");
    fs::write(&file_path, b"single file content").unwrap();

    let result = delete_path(&file_path, DeleteMode::Permanent);
    assert!(result.is_ok(), "delete_path failed on single file: {:?}", result);
    assert!(!file_path.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_delete_path_trash_mode() {
    let temp_dir = std::env::temp_dir().join("degunk_test_trash_mode");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("trash_me.txt");
    fs::write(&file_path, b"recycle bin candidate").unwrap();

    let result = delete_path(&temp_dir, DeleteMode::Trash);
    if result.is_ok() {
        assert!(!temp_dir.exists());
    } else {
        let err = result.unwrap_err();
        assert_eq!(err.target_path, temp_dir);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
