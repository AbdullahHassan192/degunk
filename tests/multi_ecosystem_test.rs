use std::fs::{self, File};
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use blackhole::core::ecosystem::Ecosystem;
use blackhole::core::scanner::{ScanMessage, Scanner};

#[test]
fn test_multi_ecosystem_scanning_and_filtering() {
    let base = std::env::temp_dir().join("bh_multi_test_env");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    // 1. Node Project
    let node_dir = base.join("web_app");
    fs::create_dir_all(node_dir.join("node_modules").join("react")).unwrap();
    let mut f = File::create(node_dir.join("package.json")).unwrap();
    writeln!(f, r#"{{"name":"web_app"}}"#).unwrap();
    let _ = File::create(node_dir.join("package-lock.json")).unwrap();
    let _ = File::create(node_dir.join("node_modules").join("react").join("index.js")).unwrap();

    // 2. Python Project
    let py_dir = base.join("ai_service");
    fs::create_dir_all(py_dir.join(".venv").join("lib")).unwrap();
    let mut f = File::create(py_dir.join("pyproject.toml")).unwrap();
    writeln!(f, r#"[tool.poetry]\nname="ai_service""#).unwrap();
    let _ = File::create(py_dir.join("poetry.lock")).unwrap();

    // 3. Next.js Project
    let next_dir = base.join("landing_page");
    fs::create_dir_all(next_dir.join(".next").join("cache")).unwrap();
    let mut f = File::create(next_dir.join("package.json")).unwrap();
    writeln!(f, r#"{{"name":"landing_page"}}"#).unwrap();
    let _ = File::create(next_dir.join("next.config.js")).unwrap();

    // Test Scan ALL
    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));
    Scanner::start_scan(vec![base.clone()], tx, cancel, None);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => found.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    assert_eq!(found.len(), 3);
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == "node_modules"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Python && a.folder_name == ".venv"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == ".next"));

    // Test Ecosystem Filtering (Python only)
    let mut py_set = std::collections::HashSet::new();
    py_set.insert(Ecosystem::Python);

    let (tx2, rx2) = crossbeam_channel::unbounded();
    let cancel2 = Arc::new(AtomicBool::new(false));
    Scanner::start_scan(vec![base.clone()], tx2, cancel2, Some(py_set));

    let mut found_py = Vec::new();
    while let Ok(msg) = rx2.recv() {
        match msg {
            ScanMessage::Found(art) => found_py.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    assert_eq!(found_py.len(), 1);
    assert_eq!(found_py[0].ecosystem, Ecosystem::Python);

    let _ = fs::remove_dir_all(&base);
}
