use std::fs::{self, File};
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use blackhole::core::ecosystem::Ecosystem;
use blackhole::core::scanner::{ScanMessage, Scanner};

#[test]
fn test_detect_zig_project() {
    let base = std::env::temp_dir().join("bh_test_zig_proj");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(base.join("zig-cache")).unwrap();
    fs::create_dir_all(base.join("zig-out")).unwrap();

    let mut f = File::create(base.join("build.zig")).unwrap();
    writeln!(f, "// zig build").unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));
    Scanner::start_scan(vec![base.clone()], tx, cancel, None, false);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => found.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Zig && a.folder_name == "zig-cache"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Zig && a.folder_name == "zig-out"));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_detect_godot_project() {
    let base = std::env::temp_dir().join("bh_test_godot_proj");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(base.join(".godot")).unwrap();

    let mut f = File::create(base.join("project.godot")).unwrap();
    writeln!(f, "config_version=5").unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));
    Scanner::start_scan(vec![base.clone()], tx, cancel, None, false);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => found.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].ecosystem, Ecosystem::Godot);
    assert_eq!(found[0].folder_name, ".godot");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_detect_unity_and_coverage() {
    let base = std::env::temp_dir().join("bh_test_unity_cov");
    let _ = fs::remove_dir_all(&base);

    // Unity project
    let unity_dir = base.join("game");
    fs::create_dir_all(unity_dir.join("ProjectSettings")).unwrap();
    fs::create_dir_all(unity_dir.join("Library")).unwrap();
    fs::create_dir_all(unity_dir.join("Temp")).unwrap();
    let mut f1 = File::create(unity_dir.join("ProjectSettings").join("ProjectVersion.txt")).unwrap();
    writeln!(f1, "m_EditorVersion: 2022.3.0f1").unwrap();

    // Coverage in node project
    let node_dir = base.join("web");
    fs::create_dir_all(node_dir.join("coverage")).unwrap();
    let mut f2 = File::create(node_dir.join("package.json")).unwrap();
    writeln!(f2, r#"{{"name":"web"}}"#).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));
    Scanner::start_scan(vec![base.clone()], tx, cancel, None, false);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => found.push(art),
            ScanMessage::Finished { .. } => break,
            _ => {}
        }
    }

    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Unity && a.folder_name == "Library"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Unity && a.folder_name == "Temp"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Coverage && a.folder_name == "coverage"));

    let _ = fs::remove_dir_all(&base);
}
