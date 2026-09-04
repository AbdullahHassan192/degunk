use std::path::Path;

#[derive(Debug, Clone, Copy, Default)]
pub struct FolderStats {
    pub bytes: u64,
    pub file_count: usize,
}

/// Recursively calculates the total disk size and file count of a directory.
pub fn calculate_dir_size(path: &Path) -> FolderStats {
    let mut total_bytes = 0u64;
    let mut file_count = 0usize;

    let walker = walkdir::WalkDir::new(path)
        .same_file_system(true)
        .follow_links(false)
        .into_iter();

    for entry in walker.filter_map(|e| e.ok()) {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                total_bytes += metadata.len();
                file_count += 1;
            }
        }
    }

    FolderStats {
        bytes: total_bytes,
        file_count,
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

        let stats = calculate_dir_size(&temp_dir);
        assert_eq!(stats.bytes, 10);
        assert_eq!(stats.file_count, 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
