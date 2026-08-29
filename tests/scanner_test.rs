use std::fs::{self, File};
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use blackhole::core::ecosystem::Ecosystem;
use blackhole::core::scanner::{ScanMessage, Scanner};

#[test]
fn test_scanner_detects_node_modules() {
    let temp_dir = std::env::temp_dir().join("bh_test_node_proj");
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

    Scanner::start_scan(vec![temp_dir.clone()], tx, cancel, None);

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
    let temp_dir = std::env::temp_dir().join("bh_test_rust_proj");
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

    Scanner::start_scan(vec![temp_dir.clone()], tx, cancel, None);

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
