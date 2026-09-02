use std::fs::{self, File};
use std::io::Write;

use blackhole::core::deleter::{delete_path, DeleteMode};

#[test]
fn test_delete_readonly_directory() {
    let temp_dir = std::env::temp_dir().join("bh_test_readonly_del");
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
