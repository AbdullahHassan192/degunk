use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathKind {
    CurrentDir,
    ProjectDir,
    Drive,
}

#[derive(Debug, Clone)]
pub struct SuggestedPath {
    pub label: String,
    pub path: PathBuf,
    pub kind: PathKind,
}

/// Normalizes a path string for deduplication without Windows \\?\ UNC prefix.
fn path_dedup_key(path: &Path) -> String {
    let s = path.to_string_lossy();
    let trimmed = if s.starts_with(r"\\?\") {
        &s[4..]
    } else {
        &s
    };
    #[cfg(target_os = "windows")]
    {
        trimmed.to_lowercase().replace('/', "\\")
    }
    #[cfg(not(target_os = "windows"))]
    {
        trimmed.to_string()
    }
}

#[cfg(target_os = "windows")]
fn detect_system_drives() -> Vec<PathBuf> {
    let mut drives = Vec::new();
    for letter in b'C'..=b'Z' {
        let drive_str = format!("{}:\\", letter as char);
        let path = PathBuf::from(&drive_str);
        if path.is_dir() {
            drives.push(path);
        }
    }
    drives
}

#[cfg(not(target_os = "windows"))]
fn detect_system_drives() -> Vec<PathBuf> {
    let mut drives = Vec::new();
    let root = PathBuf::from("/");
    if root.is_dir() {
        drives.push(root);
    }
    drives
}

/// Filters out parent directories if a more specific child directory candidate exists.
pub fn filter_parent_directories(candidates: Vec<(String, PathBuf)>) -> Vec<(String, PathBuf)> {
    let mut filtered = Vec::new();
    for (label, path) in &candidates {
        let is_parent_of_other = candidates.iter().any(|(_, other_path)| {
            other_path != path && other_path.starts_with(path)
        });
        if !is_parent_of_other {
            filtered.push((label.clone(), path.clone()));
        }
    }
    filtered
}

/// Detects sensible default scan locations: current directory, common dev folders, and system drives.
pub fn detect_suggested_paths() -> Vec<SuggestedPath> {
    let mut suggestions = Vec::new();
    let mut seen = HashSet::new();

    // 1. Current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let key = path_dedup_key(&cwd);
        seen.insert(key);
        suggestions.push(SuggestedPath {
            label: "Current Directory".to_string(),
            path: cwd,
            kind: PathKind::CurrentDir,
        });
    }

    // 2. Common project directories in home folder
    let mut dev_candidates = Vec::new();

    if let Some(home) = dirs::home_dir() {
        let candidates = [
            ("Projects", home.join("Projects")),
            ("Projects", home.join("projects")),
            ("Dev", home.join("Dev")),
            ("Dev", home.join("dev")),
            ("Code", home.join("Code")),
            ("Code", home.join("code")),
            ("Source Repos", home.join("source").join("repos")),
            ("Source", home.join("source")),
            ("Workspace", home.join("Workspace")),
            ("Workspace", home.join("workspace")),
            ("Desktop", home.join("Desktop")),
            ("Documents", home.join("Documents")),
        ];

        for (label, path) in candidates {
            if path.is_dir() {
                dev_candidates.push((label.to_string(), path));
            }
        }
    }

    // 3. Common project roots on system drives
    let drives = detect_system_drives();
    for drive in &drives {
        let drive_name = drive.display().to_string();
        let drive_trimmed = drive_name.trim_end_matches('\\').trim_end_matches('/');
        let root_candidates = [
            (format!("Projects ({})", drive_trimmed), drive.join("Projects")),
            (format!("Projects ({})", drive_trimmed), drive.join("projects")),
            (format!("Dev ({})", drive_trimmed), drive.join("Dev")),
            (format!("Dev ({})", drive_trimmed), drive.join("dev")),
            (format!("Code ({})", drive_trimmed), drive.join("Code")),
            (format!("Code ({})", drive_trimmed), drive.join("code")),
            (format!("Workspace ({})", drive_trimmed), drive.join("workspace")),
        ];
        for (label, path) in root_candidates {
            if path.is_dir() {
                dev_candidates.push((label, path));
            }
        }
    }

    // Deduplicate parent-child dev folders: prefer specific subfolders (e.g. ~/source/repos over ~/source)
    let filtered_dev = filter_parent_directories(dev_candidates);

    for (label, path) in filtered_dev {
        let key = path_dedup_key(&path);
        if seen.insert(key) {
            suggestions.push(SuggestedPath {
                label,
                path,
                kind: PathKind::ProjectDir,
            });
        }
    }

    // 4. System drives / Root partitions
    for drive in drives {
        let key = path_dedup_key(&drive);
        if seen.insert(key) {
            let label = format!("Drive ({})", drive.display());
            suggestions.push(SuggestedPath {
                label,
                path: drive,
                kind: PathKind::Drive,
            });
        }
    }

    suggestions
}

/// Expands environment variables (%VAR% or $VAR / ${VAR}) and tilde (~) in a path string.
pub fn expand_path_str(input: &str) -> String {
    let trimmed = input.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut result = trimmed.to_string();

    // 1. Expand Windows-style %VAR%
    if result.contains('%') {
        let mut expanded = String::new();
        let mut chars = result.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '%' {
                let mut var_name = String::new();
                let mut closed = false;
                for next_ch in chars.by_ref() {
                    if next_ch == '%' {
                        closed = true;
                        break;
                    }
                    var_name.push(next_ch);
                }
                if closed && !var_name.is_empty() {
                    if let Ok(val) = std::env::var(&var_name) {
                        expanded.push_str(&val);
                    } else {
                        expanded.push('%');
                        expanded.push_str(&var_name);
                        expanded.push('%');
                    }
                } else {
                    expanded.push('%');
                    expanded.push_str(&var_name);
                }
            } else {
                expanded.push(ch);
            }
        }
        result = expanded;
    }

    // 2. Expand Unix-style ${VAR} and $VAR
    if result.contains('$') {
        let mut expanded = String::new();
        let mut chars = result.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '$' {
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume '{'
                    let mut var_name = String::new();
                    let mut closed = false;
                    for next_ch in chars.by_ref() {
                        if next_ch == '}' {
                            closed = true;
                            break;
                        }
                        var_name.push(next_ch);
                    }
                    if closed && !var_name.is_empty() {
                        if let Ok(val) = std::env::var(&var_name) {
                            expanded.push_str(&val);
                        } else {
                            expanded.push_str("${");
                            expanded.push_str(&var_name);
                            expanded.push('}');
                        }
                    } else {
                        expanded.push_str("${");
                        expanded.push_str(&var_name);
                    }
                } else {
                    let mut var_name = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            var_name.push(next_ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if !var_name.is_empty() {
                        if let Ok(val) = std::env::var(&var_name) {
                            expanded.push_str(&val);
                        } else {
                            expanded.push('$');
                            expanded.push_str(&var_name);
                        }
                    } else {
                        expanded.push('$');
                    }
                }
            } else {
                expanded.push(ch);
            }
        }
        result = expanded;
    }

    // 3. Expand tilde (~) at start of path
    if result == "~" || result.starts_with("~/") || result.starts_with("~\\") {
        let home = dirs::home_dir().or_else(|| {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
                .map(PathBuf::from)
        });

        if let Some(home_path) = home {
            if result == "~" {
                result = home_path.to_string_lossy().to_string();
            } else {
                let rest = &result[2..];
                let joined = home_path.join(rest);
                result = joined.to_string_lossy().to_string();
            }
        }
    }

    result
}

/// Resolves and validates a user-entered path string.
/// Handles tildes (~), environment variables (%VAR%, $VAR), surrounding quotes, and relative paths.
pub fn resolve_user_path(input: &str) -> Option<PathBuf> {
    let trimmed = input.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() {
        return None;
    }

    let expanded_str = expand_path_str(trimmed);
    if expanded_str.is_empty() {
        return None;
    }

    let path = PathBuf::from(expanded_str);
    let full_path = if path.is_relative() {
        if let Ok(cwd) = std::env::current_dir() {
            cwd.join(path)
        } else {
            path
        }
    } else {
        path
    };

    if full_path.is_dir() {
        Some(full_path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_suggested_paths_not_empty() {
        let paths = detect_suggested_paths();
        assert!(!paths.is_empty());
        assert_eq!(paths[0].kind, PathKind::CurrentDir);
    }

    #[test]
    fn test_parent_child_filtering() {
        let list = vec![
            ("Source".to_string(), PathBuf::from("/user/source")),
            ("Source Repos".to_string(), PathBuf::from("/user/source/repos")),
            ("Desktop".to_string(), PathBuf::from("/user/Desktop")),
        ];
        let filtered = filter_parent_directories(list);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|(l, _)| l == "Source Repos"));
        assert!(filtered.iter().any(|(l, _)| l == "Desktop"));
        assert!(!filtered.iter().any(|(l, _)| l == "Source"));
    }

    #[test]
    fn test_resolve_user_path_valid() {
        let cwd = std::env::current_dir().unwrap();
        let res = resolve_user_path(".");
        assert_eq!(res, Some(cwd));
    }

    #[test]
    fn test_resolve_user_path_quoted() {
        let cwd = std::env::current_dir().unwrap();
        let quoted = format!("\"{}\"", cwd.display());
        let res = resolve_user_path(&quoted);
        assert_eq!(res, Some(cwd));
    }

    #[test]
    fn test_resolve_user_path_nonexistent() {
        let res = resolve_user_path("non_existent_folder_xyz_12345");
        assert_eq!(res, None);
    }

    #[test]
    fn test_resolve_user_path_tilde() {
        if let Some(home) = dirs::home_dir() {
            let res = resolve_user_path("~");
            assert_eq!(res, Some(home));
        }
    }

    #[test]
    fn test_resolve_user_path_env_var() {
        #[cfg(target_os = "windows")]
        {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let res = resolve_user_path("%USERPROFILE%");
                assert_eq!(res, Some(PathBuf::from(userprofile)));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(home) = std::env::var("HOME") {
                let res = resolve_user_path("$HOME");
                assert_eq!(res, Some(PathBuf::from(home)));
            }
        }
    }

    #[test]
    fn test_expand_path_str_tilde_and_env() {
        let expanded = expand_path_str("~");
        assert!(!expanded.is_empty());
        assert_ne!(expanded, "~");
    }
}
