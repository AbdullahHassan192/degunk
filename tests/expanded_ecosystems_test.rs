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
fn test_detect_unreal_android_reactnative_embedded() {
    let base = std::env::temp_dir().join("degunk_test_unreal_android_rn");
    let _ = fs::remove_dir_all(&base);

    // Unreal Engine project
    let ue_dir = base.join("ShooterGame");
    fs::create_dir_all(ue_dir.join("Intermediate")).unwrap();
    fs::create_dir_all(ue_dir.join("Saved")).unwrap();
    fs::create_dir_all(ue_dir.join("DerivedDataCache")).unwrap();
    fs::create_dir_all(ue_dir.join("Binaries")).unwrap();
    let _ = File::create(ue_dir.join("ShooterGame.uproject")).unwrap();

    // Android NDK project
    let android_dir = base.join("android_ndk_app");
    fs::create_dir_all(android_dir.join(".cxx")).unwrap();
    fs::create_dir_all(android_dir.join(".externalNativeBuild")).unwrap();
    let _ = File::create(android_dir.join("build.gradle")).unwrap();
    let _ = File::create(android_dir.join("CMakeLists.txt")).unwrap();

    // React Native / Expo project
    let rn_dir = base.join("expo_mobile");
    fs::create_dir_all(rn_dir.join(".expo")).unwrap();
    fs::create_dir_all(rn_dir.join(".metro-health-check")).unwrap();
    let _ = File::create(rn_dir.join("app.json")).unwrap();
    let _ = File::create(rn_dir.join("package.json")).unwrap();

    // PlatformIO / Embedded
    let pio_dir = base.join("iot_firmware");
    fs::create_dir_all(pio_dir.join(".pio")).unwrap();
    let _ = File::create(pio_dir.join("platformio.ini")).unwrap();

    // vcpkg project
    let vcpkg_dir = base.join("cpp_vcpkg_app");
    fs::create_dir_all(vcpkg_dir.join("vcpkg_installed")).unwrap();
    let _ = File::create(vcpkg_dir.join("vcpkg.json")).unwrap();

    // Modern Web / Serverless (.output, .swc, .wrangler, .vercel, project .cache)
    let web_dir = base.join("modern_nuxt_app");
    fs::create_dir_all(web_dir.join(".output")).unwrap();
    fs::create_dir_all(web_dir.join(".swc")).unwrap();
    fs::create_dir_all(web_dir.join(".wrangler")).unwrap();
    fs::create_dir_all(web_dir.join(".vercel")).unwrap();
    fs::create_dir_all(web_dir.join(".cache")).unwrap();
    let _ = File::create(web_dir.join("package.json")).unwrap();
    let _ = File::create(web_dir.join("wrangler.toml")).unwrap();

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

    // Unreal checks
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Unreal && a.folder_name == "Intermediate"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Unreal && a.folder_name == "Saved"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Unreal && a.folder_name == "DerivedDataCache"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Unreal && a.folder_name == "Binaries"));

    // Android NDK checks
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Android && a.folder_name == ".cxx"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Android && a.folder_name == ".externalNativeBuild"));

    // React Native checks
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::ReactNative && a.folder_name == ".expo"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::ReactNative && a.folder_name == ".metro-health-check"));

    // PlatformIO check
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Embedded && a.folder_name == ".pio"));

    // vcpkg check
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Cpp && a.folder_name == "vcpkg_installed"));

    // Modern Web / Cloudflare / Project .cache checks
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == ".output"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == ".swc"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == ".wrangler"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == ".vercel"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Node && a.folder_name == ".cache"));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_detect_dotnet_benchmarks_r_elm_clojure_julia() {
    let base = std::env::temp_dir().join("degunk_test_dotnet_r_elm_clj_jl");
    let _ = fs::remove_dir_all(&base);

    // .NET Benchmarks & TestResults
    let dotnet_dir = base.join("DotNetPerf");
    fs::create_dir_all(dotnet_dir.join("BenchmarkDotNet.Artifacts")).unwrap();
    fs::create_dir_all(dotnet_dir.join("TestResults")).unwrap();
    let _ = File::create(dotnet_dir.join("DotNetPerf.csproj")).unwrap();

    // R project with .Rproj.user and renv
    let r_dir = base.join("RProject");
    fs::create_dir_all(r_dir.join(".Rproj.user")).unwrap();
    fs::create_dir_all(r_dir.join("renv").join("library")).unwrap();
    let _ = File::create(r_dir.join("analysis.Rproj")).unwrap();
    let _ = File::create(r_dir.join("renv.lock")).unwrap();

    // Elm project
    let elm_dir = base.join("ElmApp");
    fs::create_dir_all(elm_dir.join("elm-stuff")).unwrap();
    let _ = File::create(elm_dir.join("elm.json")).unwrap();

    // Clojure project
    let clj_dir = base.join("ClojureService");
    fs::create_dir_all(clj_dir.join(".cpcache")).unwrap();
    fs::create_dir_all(clj_dir.join(".shadow-cljs")).unwrap();
    let _ = File::create(clj_dir.join("deps.edn")).unwrap();

    // Julia project
    let jl_dir = base.join("JuliaPackage");
    fs::create_dir_all(jl_dir.join(".julia")).unwrap();
    let _ = File::create(jl_dir.join("Project.toml")).unwrap();

    // CMake debug/release builds
    let cmake_dir = base.join("CppEngine");
    fs::create_dir_all(cmake_dir.join("cmake-build-debug")).unwrap();
    fs::create_dir_all(cmake_dir.join("cmake-build-release")).unwrap();
    let _ = File::create(cmake_dir.join("CMakeLists.txt")).unwrap();

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

    // .NET checks
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::DotNet && a.folder_name == "BenchmarkDotNet.Artifacts"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::DotNet && a.folder_name == "TestResults"));

    // R checks
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::R && a.folder_name == ".Rproj.user"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::R && a.folder_name == "library"));

    // Elm check
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Elm && a.folder_name == "elm-stuff"));

    // Clojure checks
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Clojure && a.folder_name == ".cpcache"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Clojure && a.folder_name == ".shadow-cljs"));

    // Julia check
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Julia && a.folder_name == ".julia"));

    // CMake extended checks
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Cpp && a.folder_name == "cmake-build-debug"));
    assert!(found.iter().any(|a| a.ecosystem == Ecosystem::Cpp && a.folder_name == "cmake-build-release"));

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

    let art_ue = DiscoveredArtifact {
        id: 3,
        target_path: PathBuf::from("/games/MyGame/Intermediate"),
        project_path: PathBuf::from("/games/MyGame"),
        project_name: "MyGame".to_string(),
        display_path: "games/MyGame".to_string(),
        root_project_path: PathBuf::from("/games/MyGame"),
        root_project_name: "MyGame".to_string(),
        sub_path: "".to_string(),
        folder_name: "Intermediate".to_string(),
        ecosystem: Ecosystem::Unreal,
        rule_label: "Unreal Engine Build & Cache".to_string(),
        reinstall_cmd: "Build".to_string(),
        size_bytes: 12 * 1024 * 1024 * 1024,
        file_count: 2400,
        size_calculated: true,
        has_lockfile: false,
        lockfile_name: None,
        activity: None,
        days_inactive: 5,
        is_git: true,
        git_clean: true,
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
    assert!(matches_artifact_query(&art_ue, "eco:unreal"));
    assert!(matches_artifact_query(&art_ue, "eco:ue5"));

    // Size filter prefix
    assert!(matches_artifact_query(&art_ruby, "size:>10m"));
    assert!(!matches_artifact_query(&art_ruby, "size:>100m"));
    assert!(matches_artifact_query(&art_tf, "size:>100m"));
    assert!(matches_artifact_query(&art_tf, "size:<500m"));
    assert!(matches_artifact_query(&art_ue, "size:>10g"));

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
        name: "Go Module Cache".to_string(),
        ecosystem: Ecosystem::Go,
        path: PathBuf::from("/Users/alice/go/pkg/mod"),
        description: "Go module sources".to_string(),
        clean_hint: "go clean -modcache".to_string(),
        size_bytes: 8 * 1024 * 1024 * 1024,
        file_count: 5000,
        size_calculated: true,
        is_selected: false,
        is_deleted: false,
    };

    assert!(matches_global_cache_query(&gc, "module"));
    assert!(matches_global_cache_query(&gc, "eco:go"));
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
