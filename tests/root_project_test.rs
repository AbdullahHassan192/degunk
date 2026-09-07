use std::fs;
use degunk::core::scanner::resolve_root_project;

#[test]
fn test_resolve_root_project_cargo_workspace() {
    let base = std::env::temp_dir().join("degunk_test_root_cargo");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let workspace_root = base.join("my_workspace");
    fs::create_dir_all(&workspace_root).unwrap();
    fs::write(workspace_root.join("Cargo.toml"), b"[workspace]\nmembers = [\"crates/*\"]").unwrap();

    let member_a = workspace_root.join("crates").join("member_a");
    fs::create_dir_all(&member_a).unwrap();
    fs::write(member_a.join("Cargo.toml"), b"[package]\nname = \"member_a\"").unwrap();

    let (root_path, root_name, sub_path) = resolve_root_project(&member_a, &base);

    assert_eq!(root_path, workspace_root);
    assert_eq!(root_name, "my_workspace");
    assert_eq!(sub_path, "crates/member_a");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_resolve_root_project_npm_workspaces() {
    let base = std::env::temp_dir().join("degunk_test_root_npm");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let monorepo_root = base.join("js_monorepo");
    fs::create_dir_all(&monorepo_root).unwrap();
    fs::write(
        monorepo_root.join("package.json"),
        br#"{"name":"js_monorepo","workspaces":["packages/*"]}"#,
    )
    .unwrap();

    let frontend = monorepo_root.join("packages").join("frontend");
    fs::create_dir_all(&frontend).unwrap();
    fs::write(
        frontend.join("package.json"),
        br#"{"name":"frontend"}"#,
    )
    .unwrap();

    let (root_path, root_name, sub_path) = resolve_root_project(&frontend, &base);

    assert_eq!(root_path, monorepo_root);
    assert_eq!(root_name, "js_monorepo");
    assert_eq!(sub_path, "packages/frontend");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_resolve_root_project_stops_at_scan_root() {
    let base = std::env::temp_dir().join("degunk_test_root_boundary");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    // Standalone project directly inside scan root
    let standalone = base.join("standalone_app");
    fs::create_dir_all(&standalone).unwrap();
    fs::write(standalone.join("Cargo.toml"), b"[package]\nname = \"standalone\"").unwrap();

    let (root_path, root_name, sub_path) = resolve_root_project(&standalone, &base);

    assert_eq!(root_path, standalone);
    assert_eq!(root_name, "standalone_app");
    assert_eq!(sub_path, "");

    let _ = fs::remove_dir_all(&base);
}
