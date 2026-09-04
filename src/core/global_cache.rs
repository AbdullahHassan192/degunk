use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::core::ecosystem::Ecosystem;
use crate::core::size::calculate_dir_size;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCacheTarget {
    pub id: usize,
    pub name: String,
    pub ecosystem: Ecosystem,
    pub path: PathBuf,
    pub description: String,
    pub clean_hint: String,
    pub size_bytes: u64,
    pub file_count: usize,
    pub size_calculated: bool,
    pub is_selected: bool,
    pub is_deleted: bool,
}

#[derive(Debug, Clone)]
pub enum GlobalCacheMessage {
    Discovered(Vec<GlobalCacheTarget>),
    SizeUpdated {
        id: usize,
        size_bytes: u64,
        file_count: usize,
    },
    Finished,
}

type CacheCandidateSpec = (
    &'static str,
    Ecosystem,
    &'static str,
    &'static str,
    Vec<Option<PathBuf>>,
);

/// Detects well-known developer tool global caches present on the system.
pub fn detect_global_caches() -> Vec<GlobalCacheTarget> {
    let mut targets = Vec::new();
    let home = dirs::home_dir();
    let cache = dirs::cache_dir();
    let config = dirs::config_dir();
    let data_local = dirs::data_local_dir();
    let mut id = 0;

    let specs: Vec<CacheCandidateSpec> = vec![
        (
            "Cargo Package Cache",
            Ecosystem::Rust,
            "Downloaded crate archives (.crate files)",
            "cargo cache -a or delete",
            vec![
                std::env::var("CARGO_HOME").ok().map(PathBuf::from).map(|p| p.join("registry")),
                home.as_ref().map(|h| h.join(".cargo").join("registry")),
            ],
        ),
        (
            "Cargo Git Checkouts",
            Ecosystem::Rust,
            "Cloned git dependencies for Rust projects",
            "cargo cache -g or delete",
            vec![
                std::env::var("CARGO_HOME").ok().map(PathBuf::from).map(|p| p.join("git")),
                home.as_ref().map(|h| h.join(".cargo").join("git")),
            ],
        ),
        (
            "Rustup Toolchains",
            Ecosystem::Rust,
            "Installed Rust toolchains (nightly, beta, and older stable releases)",
            "rustup toolchain uninstall <name> or delete unused",
            vec![
                std::env::var("RUSTUP_HOME").ok().map(PathBuf::from).map(|p| p.join("toolchains")),
                home.as_ref().map(|h| h.join(".rustup").join("toolchains")),
            ],
        ),
        (
            "sccache Compiler Cache",
            Ecosystem::Rust,
            "Shared compiler cache for Rust, C, and C++",
            "sccache --stop-server && delete or sccache --zero-stats",
            vec![
                std::env::var("SCCACHE_DIR").ok().map(PathBuf::from),
                data_local.as_ref().map(|d| d.join("Mozilla").join("sccache")),
                cache.as_ref().map(|c| c.join("sccache")),
                home.as_ref().map(|h| h.join(".cache").join("sccache")),
            ],
        ),
        (
            "npm Cache",
            Ecosystem::Node,
            "Downloaded npm package tarballs and metadata",
            "npm cache clean --force",
            vec![
                cache.as_ref().map(|c| c.join("npm-cache")),
                config.as_ref().map(|c| c.join("npm-cache")),
                home.as_ref().map(|h| h.join(".npm")),
            ],
        ),
        (
            "pnpm Global Store",
            Ecosystem::Node,
            "Content-addressable package store",
            "pnpm store prune",
            vec![
                data_local.as_ref().map(|d| d.join("pnpm").join("store")),
                home.as_ref().map(|h| h.join("Library").join("pnpm").join("store")),
                home.as_ref().map(|h| h.join(".local").join("share").join("pnpm").join("store")),
            ],
        ),
        (
            "Yarn Cache",
            Ecosystem::Node,
            "Downloaded Yarn packages and metadata",
            "yarn cache clean",
            vec![
                cache.as_ref().map(|c| c.join("Yarn").join("Cache")),
                cache.as_ref().map(|c| c.join("yarn")),
                cache.as_ref().map(|c| c.join("Yarn")),
            ],
        ),
        (
            "Bun Package Cache",
            Ecosystem::Node,
            "Downloaded npm tarballs and git repos for Bun",
            "bun pm cache rm",
            vec![
                home.as_ref().map(|h| h.join(".bun").join("install").join("cache")),
                cache.as_ref().map(|c| c.join("bun").join("install").join("cache")),
            ],
        ),
        (
            "Playwright Browser Binaries",
            Ecosystem::Node,
            "Downloaded Chromium, Firefox, and WebKit browser builds",
            "npx playwright uninstall --all or delete",
            vec![
                std::env::var("PLAYWRIGHT_BROWSERS_PATH").ok().map(PathBuf::from),
                data_local.as_ref().map(|d| d.join("ms-playwright")),
                cache.as_ref().map(|c| c.join("ms-playwright")),
                home.as_ref().map(|h| h.join("Library").join("Caches").join("ms-playwright")),
                home.as_ref().map(|h| h.join(".cache").join("ms-playwright")),
            ],
        ),
        (
            "Cypress Binary Cache",
            Ecosystem::Node,
            "Downloaded desktop browser binaries and cache for Cypress",
            "npx cypress cache clear or delete",
            vec![
                std::env::var("CYPRESS_CACHE_FOLDER").ok().map(PathBuf::from),
                data_local.as_ref().map(|d| d.join("Cypress").join("Cache")),
                cache.as_ref().map(|c| c.join("Cypress")),
                home.as_ref().map(|h| h.join("Library").join("Caches").join("Cypress")),
                home.as_ref().map(|h| h.join(".cache").join("Cypress")),
            ],
        ),
        (
            "pip Cache",
            Ecosystem::Python,
            "Cached wheels and source archives for pip",
            "pip cache purge",
            vec![
                cache.as_ref().map(|c| c.join("pip").join("Cache")),
                cache.as_ref().map(|c| c.join("pip")),
            ],
        ),
        (
            "uv Cache",
            Ecosystem::Python,
            "Fast Python package and environment cache",
            "uv cache clean",
            vec![
                cache.as_ref().map(|c| c.join("uv").join("cache")),
                cache.as_ref().map(|c| c.join("uv")),
            ],
        ),
        (
            "Ollama Model Weights",
            Ecosystem::Python,
            "Downloaded local LLM model weights and blobs",
            "ollama rm <model> or delete",
            vec![
                std::env::var("OLLAMA_MODELS").ok().map(PathBuf::from),
                home.as_ref().map(|h| h.join(".ollama").join("models")),
            ],
        ),
        (
            "LM Studio Model Weights",
            Ecosystem::Python,
            "Downloaded local LLMs and GGUFs via LM Studio",
            "Delete models in LM Studio or delete folder",
            vec![
                cache.as_ref().map(|c| c.join("lm-studio").join("models")),
                home.as_ref().map(|h| h.join(".lmstudio").join("models")),
            ],
        ),
        (
            "Whisper AI Model Cache",
            Ecosystem::Python,
            "Downloaded OpenAI Whisper model checkpoints",
            "Delete unused model weights",
            vec![
                cache.as_ref().map(|c| c.join("whisper")),
                home.as_ref().map(|h| h.join(".cache").join("whisper")),
            ],
        ),
        (
            "HuggingFace Hub Cache",
            Ecosystem::Python,
            "Cached models, datasets and tokenizer weights",
            "huggingface-cli delete-cache or delete",
            vec![
                std::env::var("HF_HOME").ok().map(PathBuf::from).map(|p| p.join("hub")),
                std::env::var("HF_HOME").ok().map(PathBuf::from),
                cache.as_ref().map(|c| c.join("huggingface").join("hub")),
                cache.as_ref().map(|c| c.join("huggingface")),
                home.as_ref().map(|h| h.join(".cache").join("huggingface").join("hub")),
                home.as_ref().map(|h| h.join(".cache").join("huggingface")),
            ],
        ),
        (
            "PyTorch Hub Cache",
            Ecosystem::Python,
            "Pretrained model checkpoints downloaded via torch.hub",
            "Delete or clear torch cache",
            vec![
                std::env::var("TORCH_HOME").ok().map(PathBuf::from),
                cache.as_ref().map(|c| c.join("torch").join("hub")),
                home.as_ref().map(|h| h.join(".cache").join("torch").join("hub")),
            ],
        ),
        (
            "Gradle Cache",
            Ecosystem::Java,
            "Downloaded jar dependencies and distribution zips",
            "Delete or rebuild via gradle",
            vec![home.as_ref().map(|h| h.join(".gradle").join("caches"))],
        ),
        (
            "Gradle Daemon Logs & State",
            Ecosystem::Java,
            "Accumulated Gradle background daemon logs and heap dumps",
            "./gradlew --stop or delete",
            vec![home.as_ref().map(|h| h.join(".gradle").join("daemon"))],
        ),
        (
            "Maven Repository",
            Ecosystem::Java,
            "Downloaded artifacts and plugins repository",
            "Delete or mvn dependency:purge-local-repository",
            vec![home.as_ref().map(|h| h.join(".m2").join("repository"))],
        ),
        (
            "Go Build Cache",
            Ecosystem::Go,
            "Compiled Go packages and build artifacts",
            "go clean -cache",
            vec![
                std::env::var("GOCACHE").ok().map(PathBuf::from),
                cache.as_ref().map(|c| c.join("go-build")),
            ],
        ),
        (
            "Go Module Cache",
            Ecosystem::Go,
            "Downloaded Go module source archives and checksums",
            "go clean -modcache",
            vec![
                std::env::var("GOPATH").ok().map(PathBuf::from).map(|p| p.join("pkg").join("mod")),
                home.as_ref().map(|h| h.join("go").join("pkg").join("mod")),
            ],
        ),
        (
            "Pub Cache (Dart/Flutter)",
            Ecosystem::Flutter,
            "Hosted and git packages downloaded by pub",
            "dart pub cache clean",
            vec![
                std::env::var("PUB_CACHE").ok().map(PathBuf::from),
                cache.as_ref().map(|c| c.join("Pub").join("Cache")),
                home.as_ref().map(|h| h.join(".pub-cache")),
            ],
        ),
        (
            "Dart Analysis Server Cache",
            Ecosystem::Flutter,
            "Symbol indexes and analysis cache for Dart/Flutter",
            "Delete folder to regenerate",
            vec![
                cache.as_ref().map(|c| c.join(".dartServer")),
                home.as_ref().map(|h| h.join(".dartServer")),
                cache.as_ref().map(|c| c.join("dartServer")),
            ],
        ),
        (
            "Android Build Cache",
            Ecosystem::Android,
            "Exploded AAR/JAR build cache for Android Studio",
            "Clean via Android Studio or delete",
            vec![
                home.as_ref().map(|h| h.join(".android").join("cache")),
                home.as_ref().map(|h| h.join(".android").join("build-cache")),
            ],
        ),
        (
            "Android Emulator AVD Images",
            Ecosystem::Android,
            "Android Virtual Device system disk images and snapshots",
            "Delete unused emulators via Android Studio Device Manager",
            vec![
                std::env::var("ANDROID_AVD_HOME").ok().map(PathBuf::from),
                home.as_ref().map(|h| h.join(".android").join("avd")),
            ],
        ),
        (
            "NuGet Package Cache",
            Ecosystem::DotNet,
            "Global packages cache for .NET / NuGet",
            "dotnet nuget locals all --clear",
            vec![
                home.as_ref().map(|h| h.join(".nuget").join("packages")),
                cache.as_ref().map(|c| c.join("NuGet").join("Cache")),
                cache.as_ref().map(|c| c.join("NuGet").join("v3-cache")),
            ],
        ),
        (
            "vcpkg Binary Archive",
            Ecosystem::Cpp,
            "Global binary package archive of precompiled C++ libraries",
            "vcpkg cache or delete folder",
            vec![
                std::env::var("VCPKG_DEFAULT_BINARY_CACHE").ok().map(PathBuf::from),
                data_local.as_ref().map(|d| d.join("vcpkg").join("archives")),
                cache.as_ref().map(|c| c.join("vcpkg").join("archives")),
                home.as_ref().map(|h| h.join(".vcpkg").join("archives")),
                home.as_ref().map(|h| h.join(".cache").join("vcpkg").join("archives")),
            ],
        ),
        (
            "vcpkg Download Cache",
            Ecosystem::Cpp,
            "Downloaded source archives and toolchain tools for vcpkg",
            "Delete downloaded archive zips",
            vec![
                data_local.as_ref().map(|d| d.join("vcpkg").join("downloads")),
                cache.as_ref().map(|c| c.join("vcpkg").join("downloads")),
                home.as_ref().map(|h| h.join(".vcpkg").join("downloads")),
            ],
        ),
        (
            "ccache Compiler Cache",
            Ecosystem::Cpp,
            "Shared C/C++ compilation cache",
            "ccache -C or delete folder",
            vec![
                std::env::var("CCACHE_DIR").ok().map(PathBuf::from),
                data_local.as_ref().map(|d| d.join("ccache")),
                cache.as_ref().map(|c| c.join("ccache")),
                home.as_ref().map(|h| h.join(".ccache")),
                home.as_ref().map(|h| h.join(".cache").join("ccache")),
            ],
        ),
        (
            "Homebrew Download Cache",
            Ecosystem::Cpp,
            "Downloaded bottle binaries and source tarballs for Homebrew",
            "brew cleanup -s",
            vec![
                std::env::var("HOMEBREW_CACHE").ok().map(PathBuf::from),
                home.as_ref().map(|h| h.join("Library").join("Caches").join("Homebrew")),
                cache.as_ref().map(|c| c.join("Homebrew")),
            ],
        ),
        (
            "Deno Cache",
            Ecosystem::Node,
            "Remote modules and npm packages cached by Deno",
            "deno clean",
            vec![
                std::env::var("DENO_DIR").ok().map(PathBuf::from),
                cache.as_ref().map(|c| c.join("deno")),
            ],
        ),
        (
            "Ruby Gems Cache",
            Ecosystem::Ruby,
            "Cached Ruby gem archives and spec indexes",
            "gem cleanup",
            vec![
                home.as_ref().map(|h| h.join(".gem").join("specs")),
                home.as_ref().map(|h| h.join(".bundle").join("cache")),
            ],
        ),
        (
            "Coursier Cache (Scala/sbt)",
            Ecosystem::Scala,
            "Downloaded Scala dependencies and coursier artifacts",
            "cs cache clean or delete",
            vec![
                cache.as_ref().map(|c| c.join("Coursier").join("cache")),
                cache.as_ref().map(|c| c.join("coursier")),
                cache.as_ref().map(|c| c.join("Coursier")),
            ],
        ),
        (
            "Haskell Stack Cache",
            Ecosystem::Haskell,
            "Precompiled snapshots and package indexes for Stack",
            "stack purge or delete",
            vec![
                home.as_ref().map(|h| h.join(".stack").join("indices")),
                home.as_ref().map(|h| h.join(".stack").join("snapshots")),
            ],
        ),
        (
            "Xcode DerivedData (Global)",
            Ecosystem::Swift,
            "Global Xcode intermediate build outputs and module caches",
            "rm -rf ~/Library/Developer/Xcode/DerivedData/*",
            vec![home.as_ref().map(|h| h.join("Library").join("Developer").join("Xcode").join("DerivedData"))],
        ),
        (
            "CocoaPods Cache",
            Ecosystem::Swift,
            "Downloaded CocoaPods spec and package archives",
            "pod cache clean --all",
            vec![
                home.as_ref().map(|h| h.join(".cocoapods").join("cache")),
                cache.as_ref().map(|c| c.join("CocoaPods")),
            ],
        ),
        (
            "Terraform Provider Cache",
            Ecosystem::Terraform,
            "Downloaded Terraform and OpenTofu provider plugins",
            "Delete unused provider plugins",
            vec![
                home.as_ref().map(|h| h.join(".terraform.d").join("plugin-cache")),
                config.as_ref().map(|c| c.join("terraform.d").join("plugin-cache")),
            ],
        ),
        (
            "Docker Desktop WSL Data",
            Ecosystem::Terraform,
            "Docker Desktop WSL2 dynamic virtual disk (ext4.vhdx)",
            "wsl --shutdown && optimize-vhd or docker system prune -a",
            vec![
                data_local.as_ref().map(|d| d.join("Docker").join("wsl").join("data")),
                data_local.as_ref().map(|d| d.join("Docker").join("wsl")),
            ],
        ),
        (
            "Unreal Engine Global DDC",
            Ecosystem::Unreal,
            "Global shared shader, asset, and texture cache for Unreal Engine",
            "Delete folder (rebuilt automatically on project open)",
            vec![
                data_local.as_ref().map(|d| d.join("UnrealEngine").join("Common").join("DerivedDataCache")),
                home.as_ref().map(|h| h.join("Library").join("Caches").join("com.epicgames.UnrealEngine").join("DerivedDataCache")),
                config.as_ref().map(|c| c.join("Epic").join("UnrealEngine").join("DerivedDataCache")),
                home.as_ref().map(|h| h.join(".config").join("Epic").join("UnrealEngine").join("DerivedDataCache")),
            ],
        ),
        (
            "Zig Compiler & Package Cache",
            Ecosystem::Zig,
            "Global precompiled package and build cache for Zig",
            "Delete folder to invalidate zig cache",
            vec![
                data_local.as_ref().map(|d| d.join("zig")),
                cache.as_ref().map(|c| c.join("zig")),
                home.as_ref().map(|h| h.join(".cache").join("zig")),
            ],
        ),
        (
            "R renv Package Cache",
            Ecosystem::R,
            "Global content-addressable package cache for R renv",
            "renv::clean() or delete folder",
            vec![
                data_local.as_ref().map(|d| d.join("renv").join("cache")),
                home.as_ref().map(|h| h.join("Library").join("Caches").join("org.R-project.R").join("R").join("renv").join("cache")),
                cache.as_ref().map(|c| c.join("R").join("renv").join("cache")),
                home.as_ref().map(|h| h.join(".local").join("share").join("renv").join("cache")),
            ],
        ),
        (
            "JetBrains System Caches",
            Ecosystem::Java,
            "IDE compiler caches, indexes, and symbol databases",
            "File -> Invalidate Caches in IDE or delete",
            vec![cache.as_ref().map(|c| c.join("JetBrains"))],
        ),
        (
            "VS Code Workspace Storage",
            Ecosystem::Node,
            "Workspace language server indexes, state, and backup trees",
            "Delete stale workspace hashes",
            vec![config.as_ref().map(|c| c.join("Code").join("User").join("workspaceStorage"))],
        ),
        (
            "Cursor Workspace Storage",
            Ecosystem::Node,
            "Cursor IDE workspace index databases and state",
            "Delete stale workspace hashes",
            vec![config.as_ref().map(|c| c.join("Cursor").join("User").join("workspaceStorage"))],
        ),
    ];

    for (name, eco, desc, hint, paths) in specs {
        for p in paths.into_iter().flatten() {
            if p.is_dir() {
                targets.push(GlobalCacheTarget {
                    id,
                    name: name.to_string(),
                    ecosystem: eco,
                    path: p,
                    description: desc.to_string(),
                    clean_hint: hint.to_string(),
                    size_bytes: 0,
                    file_count: 0,
                    size_calculated: false,
                    is_selected: false,
                    is_deleted: false,
                });
                id += 1;
                break; // Only match first existing candidate for this spec
            }
        }
    }

    targets
}

/// Spawns background worker to calculate sizes for detected global caches.
pub fn start_global_cache_scan(
    targets: Vec<GlobalCacheTarget>,
    tx: Sender<GlobalCacheMessage>,
    cancel_flag: Arc<AtomicBool>,
) {
    use rayon::prelude::*;
    let _ = tx.send(GlobalCacheMessage::Discovered(targets.clone()));

    thread::spawn(move || {
        targets.into_par_iter().for_each(|target| {
            if cancel_flag.load(Ordering::Relaxed) {
                return;
            }
            let stats = calculate_dir_size(&target.path);
            let _ = tx.send(GlobalCacheMessage::SizeUpdated {
                id: target.id,
                size_bytes: stats.bytes,
                file_count: stats.file_count,
            });
        });

        let _ = tx.send(GlobalCacheMessage::Finished);
    });
}
