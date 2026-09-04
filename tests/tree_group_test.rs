use std::fs::{self, File};
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use degunk::core::scanner::{ScanMessage, Scanner};
use degunk::ui::app::{App, TableItem};

#[test]
fn test_project_tree_grouping_and_collapsing() {
    let base = std::env::temp_dir().join("degunk_tree_test_env");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    // 1. Create a Next.js project with both node_modules and .next
    let next_proj = base.join("my_next_app");
    fs::create_dir_all(next_proj.join("node_modules").join("react")).unwrap();
    fs::create_dir_all(next_proj.join(".next").join("cache")).unwrap();
    let mut f1 = File::create(next_proj.join("package.json")).unwrap();
    writeln!(f1, r#"{{"name":"my_next_app"}}"#).unwrap();
    let _ = File::create(next_proj.join("next.config.js")).unwrap();
    let _ = File::create(next_proj.join("package-lock.json")).unwrap();

    // 2. Create a Flutter app with root build + .dart_tool and nested android/.gradle
    let flutter_proj = base.join("my_flutter_app");
    fs::create_dir_all(flutter_proj.join("build").join("app")).unwrap();
    fs::create_dir_all(flutter_proj.join(".dart_tool")).unwrap();
    fs::create_dir_all(flutter_proj.join("android").join(".gradle")).unwrap();
    let mut f2 = File::create(flutter_proj.join("pubspec.yaml")).unwrap();
    writeln!(f2, "name: my_flutter_app").unwrap();
    let _ = File::create(flutter_proj.join("pubspec.lock")).unwrap();
    let _ = File::create(flutter_proj.join("android").join("build.gradle")).unwrap();

    // Scan
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

    // Should find 2 targets in next_proj (node_modules, .next) + 3 targets in flutter_proj (build, .dart_tool, .gradle)
    assert_eq!(found.len(), 5);

    // Initialize App with found artifacts
    let mut app = App::new(vec![base.clone()], None, false);
    app.artifacts = found;

    // By default, groups should be expanded
    let items = app.get_visible_table_items();
    // 2 groups (my_next_app with 2 children, my_flutter_app with 3 children) -> 2 headers + 5 children = 7 items
    let header_count = items.iter().filter(|i| matches!(i, TableItem::GroupHeader { .. })).count();
    let child_count = items.iter().filter(|i| matches!(i, TableItem::ChildArtifact { .. })).count();
    assert_eq!(header_count, 2);
    assert_eq!(child_count, 5);

    // Test Group Spacebar Selection: toggling group header selects all its children
    app.selected_table_index = 0; // First group header
    app.toggle_selection();

    let (selected_count, _) = app.get_selected_stats();
    assert!(selected_count > 0);

    // Test Collapse Group
    app.toggle_expand();
    let items_after_collapse = app.get_visible_table_items();
    assert!(items_after_collapse.len() < items.len());

    // Test Expand All / Collapse All
    app.toggle_expand_all();
    let items_collapsed_all = app.get_visible_table_items();
    assert_eq!(items_collapsed_all.len(), 2); // only the 2 group headers visible!

    app.toggle_expand_all();
    let items_expanded_all = app.get_visible_table_items();
    assert_eq!(items_expanded_all.len(), 7); // all 7 visible again!

    let _ = fs::remove_dir_all(&base);
}
