use std::fs::{self, File};
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use degunk::core::ecosystem::Ecosystem;
use degunk::core::scanner::{ScanMessage, Scanner};

#[test]
fn test_scanner_detects_node_modules() {
    let temp_dir = std::env::temp_dir().join("degunk_test_node_proj");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create package.json and node_modules
    let pkg_json = temp_dir.join("package.json");
    let mut f = File::create(&pkg_json).unwrap();
    writeln!(f, r#"{{"name": "test-app"}}"#).unwrap();

    let nm_dir = temp_dir.join("node_modules").join("dummy-pkg");
    fs::create_dir_all(&nm_dir).unwrap();
    let mut dummy_file = File::create(nm_dir.join("index.js")).unwrap();
    writeln!(dummy_file, "console.log('hello');").unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));

    Scanner::start_scan(vec![temp_dir.clone()], tx, cancel, None, false);

    let mut found_artifacts = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => found_artifacts.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    assert_eq!(found_artifacts.len(), 1);
    assert_eq!(found_artifacts[0].ecosystem, Ecosystem::Node);
    assert_eq!(found_artifacts[0].folder_name, "node_modules");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_scanner_detects_rust_target() {
    let temp_dir = std::env::temp_dir().join("degunk_test_rust_proj");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create Cargo.toml and target directory
    let cargo_toml = temp_dir.join("Cargo.toml");
    let mut f = File::create(&cargo_toml).unwrap();
    writeln!(f, r#"[package]\nname = "test_rust"\nversion = "0.1.0""#).unwrap();

    let target_debug = temp_dir.join("target").join("debug");
    fs::create_dir_all(&target_debug).unwrap();
    let mut dummy_bin = File::create(target_debug.join("test.exe")).unwrap();
    writeln!(dummy_bin, "fake-binary-data").unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));

    Scanner::start_scan(vec![temp_dir.clone()], tx, cancel, None, false);

    let mut found_artifacts = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => found_artifacts.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    assert_eq!(found_artifacts.len(), 1);
    assert_eq!(found_artifacts[0].ecosystem, Ecosystem::Rust);
    assert_eq!(found_artifacts[0].folder_name, "target");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_scanner_skips_onedrive_folders() {
    let temp_dir = std::env::temp_dir().join("degunk_test_onedrive_skip");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create a fake OneDrive folder with node_modules inside
    let onedrive_folder = temp_dir.join("OneDrive - Personal").join("my_cloud_proj");
    fs::create_dir_all(onedrive_folder.join("node_modules")).unwrap();
    let mut f = File::create(onedrive_folder.join("package.json")).unwrap();
    writeln!(f, r#"{{"name":"cloud_proj"}}"#).unwrap();

    // Create a normal local project outside OneDrive
    let local_folder = temp_dir.join("local_proj");
    fs::create_dir_all(local_folder.join("node_modules")).unwrap();
    let mut f2 = File::create(local_folder.join("package.json")).unwrap();
    writeln!(f2, r#"{{"name":"local_proj"}}"#).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));

    // Scan with include_cloud = false (default)
    Scanner::start_scan(vec![temp_dir.clone()], tx, cancel, None, false);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => found.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    // Should only find the local_proj, completely skipping OneDrive!
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].project_name, "local_proj");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_scanner_skips_unix_system_dirs() {
    let temp_dir = std::env::temp_dir().join("degunk_test_unix_sys_skip");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Create proc, sys, dev, run dirs with fake project artifacts inside
    for sys_dir in ["proc", "sys", "dev", "run"] {
        let dir = temp_dir.join(sys_dir).join("fake_proj");
        fs::create_dir_all(dir.join("node_modules")).unwrap();
        let mut f = File::create(dir.join("package.json")).unwrap();
        writeln!(f, r#"{{"name":"sys_proj"}}"#).unwrap();
    }

    // Create a normal valid project
    let valid_dir = temp_dir.join("workspace").join("real_proj");
    fs::create_dir_all(valid_dir.join("node_modules")).unwrap();
    let mut f = File::create(valid_dir.join("package.json")).unwrap();
    writeln!(f, r#"{{"name":"real_proj"}}"#).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));

    Scanner::start_scan(vec![temp_dir.clone()], tx, cancel, None, false);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => found.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].project_name, "real_proj");

    let _ = fs::remove_dir_all(&temp_dir);
}
