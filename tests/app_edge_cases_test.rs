use std::path::PathBuf;
use degunk::core::ecosystem::Ecosystem;
use degunk::core::scanner::DiscoveredArtifact;
use degunk::ui::app::{matches_artifact_query, ActiveTab, App, SortMode, TableItem};

fn create_mock_artifact(
    id: usize,
    name: &str,
    ecosystem: Ecosystem,
    size_bytes: u64,
    days_inactive: u32,
) -> DiscoveredArtifact {
    DiscoveredArtifact {
        id,
        target_path: PathBuf::from(format!("/workspace/{}/build", name)),
        project_path: PathBuf::from(format!("/workspace/{}", name)),
        project_name: name.to_string(),
        display_path: format!("workspace/{}", name),
        root_project_path: PathBuf::from(format!("/workspace/{}", name)),
        root_project_name: name.to_string(),
        sub_path: String::new(),
        folder_name: "build".to_string(),
        ecosystem,
        rule_label: "Mock build".to_string(),
        reinstall_cmd: "build".to_string(),
        size_bytes,
        file_count: 10,
        size_calculated: true,
        has_lockfile: true,
        lockfile_name: Some("lock.json".to_string()),
        activity: None,
        days_inactive,
        is_git: true,
        git_clean: true,
        is_selected: false,
        is_deleted: false,
    }
}

#[test]
fn test_app_empty_navigation_no_panic() {
    let mut app = App::new(vec![], None, false, true);

    // Projects tab with empty artifacts
    app.active_tab = ActiveTab::Projects;
    app.artifacts.clear();
    app.selected_table_index = 0;
    app.move_up();
    app.move_down();
    app.page_up(5);
    app.page_down(5);
    app.move_to_top();
    app.move_to_bottom();
    app.toggle_selection();
    app.toggle_all();
    app.cycle_sort();
    assert_eq!(app.selected_table_index, 0);

    // Global Caches tab with empty caches
    app.active_tab = ActiveTab::GlobalCaches;
    app.global_caches.clear();
    app.selected_cache_index = 0;
    app.move_up();
    app.move_down();
    app.page_up(5);
    app.page_down(5);
    app.move_to_top();
    app.move_to_bottom();
    app.toggle_selection();
    app.toggle_all();
    app.cycle_sort();
    assert_eq!(app.selected_cache_index, 0);
}

#[test]
fn test_app_query_parsing_edge_cases() {
    let art = create_mock_artifact(1, "omega", Ecosystem::Rust, 50 * 1024 * 1024, 15);

    // Empty and whitespace queries match everything
    assert!(matches_artifact_query(&art, ""));
    assert!(matches_artifact_query(&art, "   "));
    assert!(matches_artifact_query(&art, "\t\n"));

    // Malformed numeric queries should not panic
    assert!(matches_artifact_query(&art, "size:>notanumber"));
    assert!(matches_artifact_query(&art, "size:<invalid_threshold"));
    assert!(matches_artifact_query(&art, "age:>abc"));
    assert!(matches_artifact_query(&art, "days:<xyz"));

    // Empty prefixes and unknown ecosystems
    assert!(matches_artifact_query(&art, "eco:"));
    assert!(!matches_artifact_query(&art, "eco:nonexistent_runtime_123"));

    // Unknown flags should fallback to token substring search
    assert!(!matches_artifact_query(&art, "locked:maybe"));
    assert!(!matches_artifact_query(&art, "git:unsure"));

    // Extra whitespace handling
    assert!(matches_artifact_query(&art, "  omega   size:>10m  "));
}

#[test]
fn test_app_sorting_all_modes() {
    let mut app = App::new(vec![], None, false, true);
    app.artifacts = vec![
        create_mock_artifact(1, "alpha", Ecosystem::Python, 500 * 1024 * 1024, 10),
        create_mock_artifact(2, "beta", Ecosystem::Node, 100 * 1024 * 1024, 50),
        create_mock_artifact(3, "gamma", Ecosystem::Rust, 1000 * 1024 * 1024, 2),
    ];

    let get_group_names = |app: &App| -> Vec<String> {
        app.get_visible_table_items()
            .into_iter()
            .filter_map(|item| match item {
                TableItem::GroupHeader { display_name, .. } => Some(display_name),
                _ => None,
            })
            .collect()
    };

    // 1. SizeDesc: gamma (1000MB) > alpha (500MB) > beta (100MB)
    app.sort_mode = SortMode::SizeDesc;
    assert_eq!(get_group_names(&app), vec!["gamma", "alpha", "beta"]);

    // 2. AgeDesc: beta (50 days) > alpha (10 days) > gamma (2 days)
    app.sort_mode = SortMode::AgeDesc;
    assert_eq!(get_group_names(&app), vec!["beta", "alpha", "gamma"]);

    // 3. NameAsc: alpha < beta < gamma
    app.sort_mode = SortMode::NameAsc;
    assert_eq!(get_group_names(&app), vec!["alpha", "beta", "gamma"]);

    // 4. EcosystemAsc: Node (beta) < Python (alpha) < Rust (gamma)
    app.sort_mode = SortMode::EcosystemAsc;
    assert_eq!(get_group_names(&app), vec!["beta", "alpha", "gamma"]);
}

#[test]
fn test_app_toggle_selection_child_and_group() {
    let mut app = App::new(vec![], None, false, true);
    app.artifacts = vec![
        create_mock_artifact(1, "project_one", Ecosystem::Node, 100 * 1024 * 1024, 5),
        create_mock_artifact(2, "project_two", Ecosystem::Rust, 200 * 1024 * 1024, 10),
    ];
    // Sort by name so project_one is first
    app.sort_mode = SortMode::NameAsc;

    // Select group header 0 (project_one)
    app.selected_table_index = 0;
    app.toggle_selection();
    assert!(app.artifacts[0].is_selected);
    assert!(!app.artifacts[1].is_selected);

    // Toggle again deselects
    app.toggle_selection();
    assert!(!app.artifacts[0].is_selected);

    // Toggle all selects both
    app.toggle_all();
    let (sel_count, sel_bytes) = app.get_selected_stats();
    assert_eq!(sel_count, 2);
    assert_eq!(sel_bytes, 300 * 1024 * 1024);
}
