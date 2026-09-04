use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    Trash,
    Permanent,
}

#[derive(Debug, Clone)]
pub enum DeleteProgressMessage {
    Progress {
        current_index: usize,
        total_count: usize,
        current_path: String,
        freed_bytes: u64,
    },
    Done {
        freed_bytes: u64,
        errors: usize,
    },
}

#[derive(Debug, Default)]
pub struct DeleteResult {
    pub succeeded: Vec<PathBuf>,
    pub failed: Vec<(PathBuf, String)>,
    pub bytes_freed: u64,
}

/// Deletes an artifact directory using the chosen mode (Trash or Permanent).
pub fn delete_path(path: &Path, mode: DeleteMode) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    match mode {
        DeleteMode::Trash => {
            trash::delete(path).map_err(|e| format!("Failed to move to Trash: {}", e))
        }
        DeleteMode::Permanent => {
            fast_permanent_remove(path)
        }
    }
}

/// Fast permanent recursive removal.
/// On Windows, renames to a temporary folder in the same parent directory first
/// so the main project directory is immediately cleared, then removes the folder.
fn fast_permanent_remove(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let temp_name = format!(
            ".degunk_tmp_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let temp_path = parent.join(&temp_name);

        // Try fast rename first
        if fs::rename(path, &temp_path).is_ok() {
            if let Err(e) = remove_dir_all_resilient(&temp_path) {
                // If deletion fails, attempt to restore the original path
                let _ = fs::rename(&temp_path, path);
                return Err(format!(
                    "Failed to delete directory {}: {}",
                    temp_path.display(),
                    e
                ));
            }
            return Ok(());
        }
    }

    // Direct removal fallback
    if path.is_dir() {
        remove_dir_all_resilient(path)
            .map_err(|e| format!("Failed to delete directory {}: {}", path.display(), e))
    } else {
        remove_file_resilient(path)
            .map_err(|e| format!("Failed to delete file {}: {}", path.display(), e))
    }
}

/// Recursively removes a directory, clearing read-only attributes if permissions fail.
fn remove_dir_all_resilient(path: &Path) -> std::io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(_) => {
            strip_readonly_recursive(path);
            fs::remove_dir_all(path)
        }
    }
}

/// Removes a single file, clearing read-only flag if needed.
fn remove_file_resilient(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(_) => {
            if let Ok(metadata) = fs::metadata(path) {
                let mut perms = metadata.permissions();
                if perms.readonly() {
                    perms.set_readonly(false);
                    let _ = fs::set_permissions(path, perms);
                }
            }
            fs::remove_file(path)
        }
    }
}

/// Strips read-only attributes recursively across all children in a directory.
fn strip_readonly_recursive(path: &Path) {
    for entry in walkdir::WalkDir::new(path)
        .same_file_system(true)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(metadata) = entry.metadata() {
            let mut perms = metadata.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = fs::set_permissions(entry.path(), perms);
            }
        }
    }
}
