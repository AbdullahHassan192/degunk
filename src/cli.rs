use clap::Parser;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::deleter::{delete_path, DeleteMode};
use crate::core::ecosystem::Ecosystem;
use crate::core::scanner::{DiscoveredArtifact, ScanMessage, Scanner};
use crate::core::size::format_bytes;

#[derive(Parser, Debug)]
#[command(
    name = "blackhole",
    version = "0.1.0",
    about = "A fast, cross-platform dependency and build artifact cleaner",
    long_about = "Black Hole scans your projects for heavy, re-downloadable dependency and build directories\n(node_modules, .venv, target, etc.) and lets you reclaim disk space safely."
)]
pub struct Cli {
    /// Paths to scan (defaults to current directory)
    #[arg(value_name = "PATHS")]
    pub paths: Vec<PathBuf>,

    /// Run in non-interactive CLI mode (list found artifacts)
    #[arg(short, long)]
    pub scan: bool,

    /// Disable the interactive TUI interface
    #[arg(long)]
    pub no_tui: bool,

    /// Output results in JSON format (useful for scripting)
    #[arg(long)]
    pub json: bool,

    /// Filter by ecosystem (e.g. "node,python,rust")
    #[arg(short = 't', long = "types", value_name = "ECOSYSTEMS")]
    pub types: Option<String>,

    /// Only show/target projects inactive for more than N days
    #[arg(long = "older-than", value_name = "DAYS")]
    pub older_than: Option<u32>,

    /// Clean all matching targets without interactive prompt
    #[arg(long = "clean-all")]
    pub clean_all: bool,

    /// Send to Recycle Bin / Trash when cleaning
    #[arg(long = "trash")]
    pub trash: bool,

    /// Permanently delete when cleaning
    #[arg(long = "permanent")]
    pub permanent: bool,

    /// Perform a dry run without actually deleting anything
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

impl Cli {
    pub fn parse_ecosystems(&self) -> Option<HashSet<Ecosystem>> {
        self.types.as_ref().map(|types_str| {
            types_str
                .split(',')
                .filter_map(|s| Ecosystem::parse(s.trim()))
                .collect()
        })
    }

    pub fn target_paths(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))]
        } else {
            self.paths.clone()
        }
    }

    pub fn should_run_tui(&self) -> bool {
        !self.scan && !self.no_tui && !self.json && !self.clean_all
    }
}

/// Runs the non-interactive CLI mode.
pub fn run_cli(cli: &Cli) {
    let target_paths = cli.target_paths();
    let allowed_ecosystems = cli.parse_ecosystems();

    if !cli.json {
        println!("Scanning for dependency & build artifacts in:");
        for p in &target_paths {
            println!("  -> {}", p.display());
        }
        println!();
    }

    let (tx, rx) = crossbeam_channel::unbounded();
    let scanner = Scanner::new();
    let cancel = scanner.cancel_handle();

    Scanner::start_scan(target_paths, tx, cancel, allowed_ecosystems);

    let mut artifacts: Vec<DiscoveredArtifact> = Vec::new();
    let mut total_bytes = 0u64;

    while let Ok(msg) = rx.recv() {
        match msg {
            ScanMessage::Found(art) => {
                artifacts.push(art);
            }
            ScanMessage::SizeUpdated {
                id,
                size_bytes,
                file_count,
            } => {
                if let Some(art) = artifacts.iter_mut().find(|a| a.id == id) {
                    art.size_bytes = size_bytes;
                    art.file_count = file_count;
                    art.size_calculated = true;
                }
            }
            ScanMessage::ActivityUpdated { id, activity } => {
                if let Some(art) = artifacts.iter_mut().find(|a| a.id == id) {
                    art.days_inactive = activity.days_inactive;
                    art.is_git = activity.is_git;
                    art.git_clean = activity.git_status == crate::core::git::GitStatus::Clean;
                    art.activity = Some(activity);
                }
            }
            ScanMessage::Progress { .. } => {}
            ScanMessage::Finished { .. } => {
                break;
            }
        }
    }

    // Filter by older_than if specified
    if let Some(min_days) = cli.older_than {
        artifacts.retain(|a| a.days_inactive >= min_days);
    }

    for a in &artifacts {
        total_bytes += a.size_bytes;
    }

    if cli.json {
        let json_out = serde_json::to_string_pretty(&artifacts).unwrap_or_else(|_| "[]".to_string());
        println!("{}", json_out);
        return;
    }

    if artifacts.is_empty() {
        println!("No cleanable artifact directories found.");
        return;
    }

    println!(
        "{:<30} {:<12} {:<18} {:<12} {:<15} {}",
        "PROJECT", "TYPE", "TARGET FOLDER", "SIZE", "INACTIVITY", "LOCKFILE"
    );
    println!("{:-<100}", "");

    for a in &artifacts {
        let lock_str = if a.has_lockfile {
            "Yes".to_string()
        } else {
            "Missing".to_string()
        };

        let inactive_str = if a.days_inactive == 0 {
            "Active today".to_string()
        } else {
            format!("{}d inactive", a.days_inactive)
        };

        println!(
            "{:<30} {:<12} {:<18} {:<12} {:<15} {}",
            truncate_str(&a.project_name, 28),
            a.ecosystem.badge(),
            truncate_str(&a.folder_name, 16),
            format_bytes(a.size_bytes),
            inactive_str,
            lock_str
        );
    }

    println!("{:-<100}", "");
    println!(
        "Found {} artifacts totaling {}\n",
        artifacts.len(),
        format_bytes(total_bytes)
    );

    if cli.clean_all {
        if cli.dry_run {
            println!("[DRY RUN] Would clean {} targets and reclaim {}.", artifacts.len(), format_bytes(total_bytes));
            return;
        }

        let mode = if cli.permanent {
            DeleteMode::Permanent
        } else {
            DeleteMode::Trash
        };

        println!("Cleaning {} artifacts using {:?} mode...", artifacts.len(), mode);
        let mut freed = 0u64;
        let mut error_count = 0usize;

        for art in &artifacts {
            match delete_path(&art.target_path, mode) {
                Ok(()) => {
                    freed += art.size_bytes;
                    println!("  [OK] Cleaned {}", art.target_path.display());
                }
                Err(e) => {
                    error_count += 1;
                    eprintln!("  [ERROR] {}: {}", art.target_path.display(), e);
                }
            }
        }

        println!("\nClean complete. Reclaimed {} ({} errors).", format_bytes(freed), error_count);
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}
