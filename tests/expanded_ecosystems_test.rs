use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use degunk::core::ecosystem::Ecosystem;
use degunk::core::global_cache::{detect_global_caches, GlobalCacheTarget};
use degunk::core::scanner::{DiscoveredArtifact, ScanMessage, Scanner};
use degunk::ui::app::{matches_artifact_query, matches_global_cache_query};

#[test]
fn test_detect_ruby_and_scala() {
    let base = std::env::temp_dir().join("degunk_test_ruby_scala");
    let _ = fs::remove_dir_all(&base);

    // Ruby project
    let ruby_dir = base.join("ruby_app");
    fs::create_dir_all(ruby_dir.join(".bundle")).unwrap();
    fs::create_dir_all(ruby_dir.join("vendor")).unwrap();
    let mut f_gem = File::create(ruby_dir.join("Gemfile")).unwrap();
    writeln!(f_gem, "source 'https://rubygems.org'").unwrap();
    let _ = File::create(ruby_dir.join("Gemfile.lock")).unwrap();

    // Scala / sbt project
    let scala_dir = base.join("scala_service");
    fs::create_dir_all(scala_dir.join("target")).unwrap();
    fs::create_dir_all(scala_dir.join(".bloop")).unwrap();
    let mut f_sbt = File::create(scala_dir.join("build.sbt")).unwrap();
    writeln!(f_sbt, "name := \"my-service\"").unwrap();

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

    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Ruby && a.folder_name == ".bundle"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Ruby && a.folder_name == "vendor"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Scala && a.folder_name == "target"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Scala && a.folder_name == ".bloop"));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_detect_haskell_ocaml_terraform() {
    let base = std::env::temp_dir().join("degunk_test_hs_ocaml_tf");
    let _ = fs::remove_dir_all(&base);

    // Haskell project
    let hs_dir = base.join("hs_app");
    fs::create_dir_all(hs_dir.join(".stack-work")).unwrap();
    fs::create_dir_all(hs_dir.join("dist-newstyle")).unwrap();
    let mut f_hs = File::create(hs_dir.join("stack.yaml")).unwrap();
    writeln!(f_hs, "resolver: lts-20.0").unwrap();

    // OCaml project
    let ocaml_dir = base.join("ocaml_lib");
    fs::create_dir_all(ocaml_dir.join("_build")).unwrap();
    let mut f_dune = File::create(ocaml_dir.join("dune-project")).unwrap();
    writeln!(f_dune, "(lang dune 3.0)").unwrap();

    // Terraform & Serverless project
    let tf_dir = base.join("infra");
    fs::create_dir_all(tf_dir.join(".terraform")).unwrap();
    let _ = File::create(tf_dir.join("main.tf")).unwrap();
    let _ = File::create(tf_dir.join(".terraform.lock.hcl")).unwrap();

    let sls_dir = base.join("serverless_app");
    fs::create_dir_all(sls_dir.join(".serverless")).unwrap();
    let mut f_sls = File::create(sls_dir.join("serverless.yml")).unwrap();
    writeln!(f_sls, "service: backend").unwrap();

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

    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Haskell && a.folder_name == ".stack-work"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Haskell && a.folder_name == "dist-newstyle"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Ocaml && a.folder_name == "_build"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Terraform && a.folder_name == ".terraform"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Terraform && a.folder_name == ".serverless"));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_detect_modern_frontend_and_python_artifacts() {
    let base = std::env::temp_dir().join("degunk_test_frontend_py");
    let _ = fs::remove_dir_all(&base);

    // Angular & Astro
    let ng_dir = base.join("angular_app");
    fs::create_dir_all(ng_dir.join(".angular")).unwrap();
    let _ = File::create(ng_dir.join("angular.json")).unwrap();

    let astro_dir = base.join("astro_site");
    fs::create_dir_all(astro_dir.join(".astro")).unwrap();
    let _ = File::create(astro_dir.join("astro.config.mjs")).unwrap();

    // Playwright / Test results
    let test_dir = base.join("e2e_tests");
    fs::create_dir_all(test_dir.join("test-results")).unwrap();
    fs::create_dir_all(test_dir.join("playwright-report")).unwrap();
    let _ = File::create(test_dir.join("playwright.config.ts")).unwrap();

    // Python build/dist & tox & egg-info
    let py_dir = base.join("my_python_pkg");
    fs::create_dir_all(py_dir.join("dist")).unwrap();
    fs::create_dir_all(py_dir.join(".tox")).unwrap();
    fs::create_dir_all(py_dir.join("my_pkg.egg-info")).unwrap();
    let _ = File::create(py_dir.join("pyproject.toml")).unwrap();

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

    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == ".angular"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == ".astro"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Coverage && a.folder_name == "test-results"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Coverage && a.folder_name == "playwright-report"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Python && a.folder_name == "dist"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Python && a.folder_name == ".tox"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Python && a.folder_name == "my_pkg.egg-info"));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_search_query_token_filters() {
    let art_ruby = DiscoveredArtifact {
        id: 1,
        target_path: PathBuf::from("/my/rails/app/vendor"),
        project_path: PathBuf::from("/my/rails/app"),
        project_name: "app".to_string(),
        display_path: "rails/app".to_string(),
        root_project_path: PathBuf::from("/my/rails"),
        root_project_name: "rails".to_string(),
        sub_path: "app".to_string(),
        folder_name: "vendor".to_string(),
        ecosystem: Ecosystem::Ruby,
        rule_label: "Ruby vendor bundle".to_string(),
        reinstall_cmd: "bundle install".to_string(),
        size_bytes: 50 * 1024 * 1024,
        file_count: 500,
        size_calculated: true,
        has_lockfile: true,
        lockfile_name: Some("Gemfile.lock".to_string()),
        activity: None,
        days_inactive: 10,
        is_git: true,
        git_clean: true,
        is_selected: false,
        is_deleted: false,
    };

    let art_tf = DiscoveredArtifact {
        id: 2,
        target_path: PathBuf::from("/infra/prod/.terraform"),
        project_path: PathBuf::from("/infra/prod"),
        project_name: "prod".to_string(),
        display_path: "infra/prod".to_string(),
        root_project_path: PathBuf::from("/infra"),
        root_project_name: "infra".to_string(),
        sub_path: "prod".to_string(),
        folder_name: ".terraform".to_string(),
        ecosystem: Ecosystem::Terraform,
        rule_label: "Terraform plugin cache".to_string(),
        reinstall_cmd: "terraform init".to_string(),
        size_bytes: 250 * 1024 * 1024,
        file_count: 300,
        size_calculated: true,
        has_lockfile: true,
        lockfile_name: Some(".terraform.lock.hcl".to_string()),
        activity: None,
        days_inactive: 60,
        is_git: true,
        git_clean: false,
        is_selected: false,
        is_deleted: false,
    };

    // Substring queries
    assert!(matches_artifact_query(&art_ruby, "ruby"));
    assert!(matches_artifact_query(&art_ruby, "vendor"));
    assert!(!matches_artifact_query(&art_ruby, "terraform"));

    // Ecosystem filter prefix
    assert!(matches_artifact_query(&art_ruby, "eco:ruby"));
    assert!(!matches_artifact_query(&art_ruby, "eco:tf"));
    assert!(matches_artifact_query(&art_tf, "eco:terraform"));
    assert!(matches_artifact_query(&art_tf, "eco:tf"));

    // Size filter prefix
    assert!(matches_artifact_query(&art_ruby, "size:>10m"));
    assert!(!matches_artifact_query(&art_ruby, "size:>100m"));
    assert!(matches_artifact_query(&art_tf, "size:>100m"));
    assert!(matches_artifact_query(&art_tf, "size:<500m"));

    // Lockfile and git filter
    assert!(matches_artifact_query(&art_ruby, "locked:yes"));
    assert!(matches_artifact_query(&art_ruby, "git:clean"));
    assert!(!matches_artifact_query(&art_tf, "git:clean"));
    assert!(matches_artifact_query(&art_tf, "git:dirty"));

    // Multi-token combo
    assert!(matches_artifact_query(&art_tf, "eco:tf size:>200m git:dirty"));
    assert!(!matches_artifact_query(&art_tf, "eco:tf size:>300m"));

    // Global cache query
    let gc = GlobalCacheTarget {
        id: 1,
        name: "Ollama Model Weights".to_string(),
        ecosystem: Ecosystem::Python,
        path: PathBuf::from("/Users/alice/.ollama/models"),
        description: "Local LLM weights".to_string(),
        clean_hint: "ollama rm".to_string(),
        size_bytes: 10 * 1024 * 1024 * 1024,
        file_count: 5,
        size_calculated: true,
        is_selected: false,
        is_deleted: false,
    };

    assert!(matches_global_cache_query(&gc, "ollama"));
    assert!(matches_global_cache_query(&gc, "eco:python"));
    assert!(matches_global_cache_query(&gc, "size:>5g"));
    assert!(!matches_global_cache_query(&gc, "size:>20g"));
}

#[test]
fn test_detect_global_caches_expanded() {
    let caches = detect_global_caches();
    for c in &caches {
        assert!(c.path.is_dir());
        assert!(!c.name.is_empty());
        assert!(!c.clean_hint.is_empty());
    }
}
