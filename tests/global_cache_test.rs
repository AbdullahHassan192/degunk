use std::fs::{self, File};
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use degunk::core::ecosystem::Ecosystem;
use degunk::core::global_cache::{
    detect_global_caches, start_global_cache_scan, GlobalCacheMessage, GlobalCacheTarget,
};

#[test]
fn test_detect_global_caches_basic() {
    // Should run without crashing and return detected caches (if developer tools are installed)
    let caches = detect_global_caches();
    for c in &caches {
        assert!(c.path.is_dir());
    }
}

#[test]
fn test_global_cache_scan_sizing() {
    let temp_cache_dir = std::env::temp_dir().join("degunk_test_fake_global_cache");
    let _ = fs::remove_dir_all(&temp_cache_dir);
    fs::create_dir_all(&temp_cache_dir).unwrap();

    let file_path = temp_cache_dir.join("cached_pkg.tar.gz");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "fake tarball data 1234567890").unwrap();
    drop(f);

    let target = GlobalCacheTarget {
        id: 42,
        name: "Test Fake Cache".to_string(),
        ecosystem: Ecosystem::Rust,
        path: temp_cache_dir.clone(),
        description: "Test cache description".to_string(),
        clean_hint: "cargo cache clean".to_string(),
        size_bytes: 0,
        file_count: 0,
        size_calculated: false,
        is_selected: false,
        is_deleted: false,
    };

    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));

    start_global_cache_scan(vec![target], tx, cancel);

    let mut size_updated = false;
    let mut updated_bytes = 0u64;

    while let Ok(msg) = rx.recv() {
        match msg {
            GlobalCacheMessage::SizeUpdated {
                id,
                size_bytes,
                file_count,
            } => {
                if id == 42 {
                    size_updated = true;
                    updated_bytes = size_bytes;
                    assert!(file_count >= 1);
                }
            }
            GlobalCacheMessage::Finished => break,
            _ => {}
        }
    }

    assert!(size_updated);
    assert!(updated_bytes > 0);

    let _ = fs::remove_dir_all(&temp_cache_dir);
}

#[test]
fn test_global_cache_sorting_and_filtering() {
    use degunk::ui::app::{ActiveTab, App, SortMode};
    use std::path::PathBuf;

    let mut app = App::new(vec![], None, false);
    app.active_tab = ActiveTab::GlobalCaches;

    app.global_caches = vec![
        GlobalCacheTarget {
            id: 1,
            name: "npm Cache".to_string(),
            ecosystem: Ecosystem::Node,
            path: PathBuf::from("C:/dummy/npm"),
            description: "npm cache".to_string(),
            clean_hint: "npm clean".to_string(),
            size_bytes: 500 * 1024 * 1024,
            file_count: 500,
            size_calculated: true,
            is_selected: false,
            is_deleted: false,
        },
        GlobalCacheTarget {
            id: 2,
            name: "Cargo Package Cache".to_string(),
            ecosystem: Ecosystem::Rust,
            path: PathBuf::from("C:/dummy/cargo"),
            description: "cargo cache".to_string(),
            clean_hint: "cargo clean".to_string(),
            size_bytes: 100 * 1024 * 1024,
            file_count: 100,
            size_calculated: true,
            is_selected: false,
            is_deleted: false,
        },
        GlobalCacheTarget {
            id: 3,
            name: "pip Cache".to_string(),
            ecosystem: Ecosystem::Python,
            path: PathBuf::from("C:/dummy/pip"),
            description: "pip cache".to_string(),
            clean_hint: "pip clean".to_string(),
            size_bytes: 50 * 1024 * 1024,
            file_count: 50,
            size_calculated: true,
            is_selected: false,
            is_deleted: false,
        },
    ];

    // 1. Sort by SizeDesc
    app.sort_mode = SortMode::SizeDesc;
    let visible = app.get_visible_global_caches();
    assert_eq!(visible.len(), 3);
    assert_eq!(visible[0].1.name, "npm Cache");
    assert_eq!(visible[1].1.name, "Cargo Package Cache");
    assert_eq!(visible[2].1.name, "pip Cache");

    // 2. Sort by NameAsc
    app.sort_mode = SortMode::NameAsc;
    let visible = app.get_visible_global_caches();
    assert_eq!(visible[0].1.name, "Cargo Package Cache");
    assert_eq!(visible[1].1.name, "npm Cache");
    assert_eq!(visible[2].1.name, "pip Cache");

    // 3. Search query: "rust" (matches ecosystem or cargo)
    app.search_query = "rust".to_string();
    let visible = app.get_visible_global_caches();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].1.name, "Cargo Package Cache");

    // 4. Search query: "cache" (matches all 3)
    app.search_query = "cache".to_string();
    let visible = app.get_visible_global_caches();
    assert_eq!(visible.len(), 3);

    // 5. Search query: "nonexistent"
    app.search_query = "nonexistent".to_string();
    let visible = app.get_visible_global_caches();
    assert_eq!(visible.len(), 0);
}
