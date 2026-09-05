use clap::Parser;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::ecosystem::Ecosystem;

#[derive(Parser, Debug)]
#[command(
    name = "degunk",
    version = "0.1.0",
    about = "A fast, interactive dependency and build artifact cleaner",
    long_about = "Degunk scans your projects for heavy, re-downloadable dependency and build directories\n(node_modules, .venv, target, etc.) and lets you reclaim disk space safely through an interactive TUI."
)]
pub struct Cli {
    /// Paths to scan (defaults to interactive path picker if omitted)
    #[arg(value_name = "PATHS")]
    pub paths: Vec<PathBuf>,

    /// Filter by ecosystem (e.g. "node,python,rust")
    #[arg(short = 't', long = "types", value_name = "ECOSYSTEMS")]
    pub types: Option<String>,

    /// Only show/target projects inactive for more than N days
    #[arg(long = "older-than", value_name = "DAYS")]
    pub older_than: Option<u32>,

    /// Minimum artifact size to display/target (e.g. "10KB", "5MB", "1GB")
    #[arg(long = "min-size", value_name = "SIZE")]
    pub min_size: Option<String>,

    /// Include cloud storage folders (OneDrive, Google Drive, Dropbox, iCloud)
    #[arg(long = "include-cloud")]
    pub include_cloud: bool,

    /// Disable startup intro animation
    #[arg(long = "no-intro")]
    pub no_intro: bool,
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

    pub fn parse_min_size_bytes(&self) -> u64 {
        if let Some(ref s) = self.min_size {
            crate::core::size::parse_size_str(s).unwrap_or(0)
        } else {
            0
        }
    }

    pub fn show_intro(&self) -> bool {
        !self.no_intro
    }

    pub fn target_paths(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))]
        } else {
            self.paths.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_paths() {
        let cli = Cli::parse_from(["degunk", "path1", "path2"]);
        assert_eq!(
            cli.paths,
            vec![PathBuf::from("path1"), PathBuf::from("path2")]
        );
        assert!(cli.show_intro());
    }

    #[test]
    fn test_cli_types_filter() {
        let cli = Cli::parse_from(["degunk", "--types", "node,rust"]);
        let ecosystems = cli.parse_ecosystems().unwrap();
        assert!(ecosystems.contains(&Ecosystem::Node));
        assert!(ecosystems.contains(&Ecosystem::Rust));
    }

    #[test]
    fn test_cli_filter_args() {
        let cli = Cli::parse_from(["degunk", "--older-than", "30", "--min-size", "50MB", "D:\\projects"]);
        assert_eq!(cli.older_than, Some(30));
        assert_eq!(cli.min_size.as_deref(), Some("50MB"));
        assert_eq!(cli.paths, vec![PathBuf::from("D:\\projects")]);
        assert_eq!(cli.parse_min_size_bytes(), 50 * 1024 * 1024);
    }

    #[test]
    fn test_cli_no_intro_flag() {
        let cli_default = Cli::parse_from(["degunk"]);
        assert!(cli_default.show_intro());

        let cli_no_intro = Cli::parse_from(["degunk", "--no-intro"]);
        assert!(!cli_no_intro.show_intro());
    }

    #[test]
    fn test_cli_removed_flags() {
        assert!(Cli::try_parse_from(["degunk", "--scan"]).is_err());
        assert!(Cli::try_parse_from(["degunk", "--no-tui"]).is_err());
        assert!(Cli::try_parse_from(["degunk", "--clean-all"]).is_err());
        assert!(Cli::try_parse_from(["degunk", "--json"]).is_err());
    }
}
