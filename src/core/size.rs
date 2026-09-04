use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FolderStats {
    pub bytes: u64,
    pub file_count: usize,
}

/// Recursively calculates the total disk size and file count of a directory
/// using parallel work-stealing directory descent via Rayon.
pub fn calculate_dir_size(path: &Path) -> FolderStats {
    let read_res = match std::fs::read_dir(path) {
        Ok(res) => res,
        Err(_) => {
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                if meta.is_file() {
                    return FolderStats {
                        bytes: meta.len(),
                        file_count: 1,
                    };
                }
            }
            return FolderStats::default();
        }
    };

    let mut local_bytes = 0u64;
    let mut local_files = 0usize;
    let mut subdirs = Vec::new();

    for entry in read_res.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if ft.is_symlink() {
            continue;
        }

        if ft.is_file() {
            if let Ok(meta) = entry.metadata() {
                local_bytes += meta.len();
                local_files += 1;
            }
        } else if ft.is_dir() {
            subdirs.push(entry.path());
        }
    }

    if subdirs.is_empty() {
        FolderStats {
            bytes: local_bytes,
            file_count: local_files,
        }
    } else if subdirs.len() == 1 {
        let sub_stats = calculate_dir_size(&subdirs[0]);
        FolderStats {
            bytes: local_bytes + sub_stats.bytes,
            file_count: local_files + sub_stats.file_count,
        }
    } else {
        use rayon::prelude::*;
        let sub_stats: FolderStats = subdirs
            .into_par_iter()
            .map(|sub| calculate_dir_size(&sub))
            .reduce(FolderStats::default, |a, b| FolderStats {
                bytes: a.bytes + b.bytes,
                file_count: a.file_count + b.file_count,
            });

        FolderStats {
            bytes: local_bytes + sub_stats.bytes,
            file_count: local_files + sub_stats.file_count,
        }
    }
}

/// Formats raw byte count into a human-readable string (e.g. "1.45 GB", "320.1 MB", "4.2 KB").
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Parses a human-readable size string (e.g. "100m", "1.5gb", "500kb", "1024") into bytes.
pub fn parse_size_str(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if let Some(num) = s.strip_suffix("tb").or_else(|| s.strip_suffix('t')) {
        num.trim().parse::<f64>().ok().map(|n| (n * TB as f64) as u64)
    } else if let Some(num) = s.strip_suffix("gb").or_else(|| s.strip_suffix('g')) {
        num.trim().parse::<f64>().ok().map(|n| (n * GB as f64) as u64)
    } else if let Some(num) = s.strip_suffix("mb").or_else(|| s.strip_suffix('m')) {
        num.trim().parse::<f64>().ok().map(|n| (n * MB as f64) as u64)
    } else if let Some(num) = s.strip_suffix("kb").or_else(|| s.strip_suffix('k')) {
        num.trim().parse::<f64>().ok().map(|n| (n * KB as f64) as u64)
    } else if let Some(num) = s.strip_suffix('b') {
        num.trim().parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(1024 * 1024 * 50), "50.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 3), "3.00 GB");
    }

    #[test]
    fn test_parse_size_str() {
        assert_eq!(parse_size_str("100m"), Some(100 * 1024 * 1024));
        assert_eq!(parse_size_str("1.5gb"), Some((1.5 * 1024.0 * 1024.0 * 1024.0) as u64));
        assert_eq!(parse_size_str("500k"), Some(500 * 1024));
        assert_eq!(parse_size_str("1024"), Some(1024));
        assert_eq!(parse_size_str("invalid"), None);
    }

    #[test]
    fn test_calculate_dir_size() {
        let temp_dir = std::env::temp_dir().join("degunk_test_size_calc");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join("a.txt"), b"12345").unwrap();
        std::fs::write(temp_dir.join("b.txt"), b"12345").unwrap();

        let sub = temp_dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("c.txt"), b"1234567890").unwrap();

        let stats = calculate_dir_size(&temp_dir);
        assert_eq!(stats.bytes, 20);
        assert_eq!(stats.file_count, 3);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
