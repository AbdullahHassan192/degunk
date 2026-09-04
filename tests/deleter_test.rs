use std::fs::{self, File};
use std::io::Write;

use degunk::core::deleter::{delete_path, delete_path_with_progress, DeleteMode};

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
    let result = delete_path_with_progress(
        &temp_dir,
        DeleteMode::Permanent,
        |bytes| {
            freed_accum += bytes;
        },
    );

    assert!(result.is_ok(), "delete_path_with_progress failed: {:?}", result);
    assert!(!temp_dir.exists(), "Target directory must be deleted");
    assert_eq!(freed_accum, expected_bytes, "Freed bytes must match expected file size sum");
}
