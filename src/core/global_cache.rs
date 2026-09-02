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

fn home_dir() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}

fn local_appdata() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
}

/// Detects well-known developer tool global caches present on the system.
pub fn detect_global_caches() -> Vec<GlobalCacheTarget> {
    let mut targets = Vec::new();
    let home = home_dir();
    let local = local_appdata();
    let mut id = 0;

    let specs: Vec<(&str, Ecosystem, &str, &str, Vec<Option<PathBuf>>)> = vec![
        (
            "Cargo Package Cache",
            Ecosystem::Rust,
            "Downloaded crate archives (.crate files)",
            "cargo cache -a or delete",
            vec![home.as_ref().map(|h| h.join(".cargo").join("registry"))],
        ),
        (
            "Cargo Git Checkouts",
            Ecosystem::Rust,
            "Cloned git dependencies for Rust projects",
            "cargo cache -g or delete",
            vec![home.as_ref().map(|h| h.join(".cargo").join("git"))],
        ),
        (
            "npm Cache",
            Ecosystem::Node,
            "Downloaded npm package tarballs and metadata",
            "npm cache clean --force",
            vec![
                local.as_ref().map(|l| l.join("npm-cache")),
                home.as_ref().map(|h| h.join(".npm")),
            ],
        ),
        (
            "pnpm Global Store",
            Ecosystem::Node,
            "Content-addressable package store",
            "pnpm store prune",
            vec![
                local.as_ref().map(|l| l.join("pnpm").join("store")),
                home.as_ref().map(|h| h.join(".local").join("share").join("pnpm").join("store")),
            ],
        ),
        (
            "Yarn Cache",
            Ecosystem::Node,
            "Downloaded Yarn packages and metadata",
            "yarn cache clean",
            vec![
                local.as_ref().map(|l| l.join("Yarn").join("Cache")),
                home.as_ref().map(|h| h.join(".cache").join("yarn")),
            ],
        ),
        (
            "pip Cache",
            Ecosystem::Python,
            "Cached wheels and source archives for pip",
            "pip cache purge",
            vec![
                local.as_ref().map(|l| l.join("pip").join("Cache")),
                home.as_ref().map(|h| h.join(".cache").join("pip")),
            ],
        ),
        (
            "uv Cache",
            Ecosystem::Python,
            "Fast Python package and environment cache",
            "uv cache clean",
            vec![
                local.as_ref().map(|l| l.join("uv").join("cache")),
                home.as_ref().map(|h| h.join(".cache").join("uv")),
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
                local.as_ref().map(|l| l.join("go-build")),
                home.as_ref().map(|h| h.join(".cache").join("go-build")),
            ],
        ),
        (
            "Pub Cache (Dart/Flutter)",
            Ecosystem::Flutter,
            "Hosted and git packages for Dart and Flutter",
            "dart pub cache clean",
            vec![
                local.as_ref().map(|l| l.join("Pub").join("Cache")),
                home.as_ref().map(|h| h.join(".pub-cache")),
            ],
        ),
        (
            "Dart Analysis Server Cache",
            Ecosystem::Flutter,
            "Symbol index and analysis driver cache for Dart/Flutter",
            "dart pub cache clean or delete",
            vec![
                local.as_ref().map(|l| l.join(".dartServer")),
                home.as_ref().map(|h| h.join(".dartServer")),
            ],
        ),
        (
            "Bun Package Cache",
            Ecosystem::Node,
            "Cached packages and modules for Bun runtime",
            "bun pm cache rm",
            vec![
                home.as_ref().map(|h| h.join(".bun").join("install").join("cache")),
                local.as_ref().map(|l| l.join("bun").join("install").join("cache")),
            ],
        ),
        (
            "Whisper AI Model Cache",
            Ecosystem::Python,
            "Downloaded OpenAI Whisper model checkpoints",
            "Delete unused model weights",
            vec![home.as_ref().map(|h| h.join(".cache").join("whisper"))],
        ),
        (
            "HuggingFace Hub Cache",
            Ecosystem::Python,
            "Cached models, datasets and tokenizer weights",
            "huggingface-cli delete-cache or delete",
            vec![
                home.as_ref().map(|h| h.join(".cache").join("huggingface").join("hub")),
                home.as_ref().map(|h| h.join(".cache").join("huggingface")),
            ],
        ),
        (
            "PyTorch Hub Cache",
            Ecosystem::Python,
            "Pretrained model checkpoints downloaded via torch.hub",
            "Delete or clear torch cache",
            vec![home.as_ref().map(|h| h.join(".cache").join("torch").join("hub"))],
        ),
        (
            "Android Build Cache",
            Ecosystem::Java,
            "Exploded AAR/JAR build cache for Android Studio",
            "Clean via Android Studio or delete",
            vec![
                home.as_ref().map(|h| h.join(".android").join("cache")),
                home.as_ref().map(|h| h.join(".android").join("build-cache")),
            ],
        ),
        (
            "NuGet Package Cache",
            Ecosystem::DotNet,
            "Global packages cache for .NET / NuGet",
            "dotnet nuget locals all --clear",
            vec![
                home.as_ref().map(|h| h.join(".nuget").join("packages")),
                local.as_ref().map(|l| l.join("NuGet").join("Cache")),
            ],
        ),
        (
            "Deno Cache",
            Ecosystem::Node,
            "Remote modules and npm packages cached by Deno",
            "deno clean",
            vec![
                local.as_ref().map(|l| l.join("deno")),
                home.as_ref().map(|h| h.join(".cache").join("deno")),
            ],
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
