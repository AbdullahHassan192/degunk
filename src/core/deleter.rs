use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    Trash,
    Permanent,
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
            ".bh_tmp_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let temp_path = parent.join(&temp_name);

        // Try fast rename first
        if fs::rename(path, &temp_path).is_ok() {
            return fs::remove_dir_all(&temp_path)
                .map_err(|e| format!("Failed to delete temporary directory {}: {}", temp_path.display(), e));
        }
    }

    // Direct removal fallback
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to delete directory {}: {}", path.display(), e))
    } else {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to delete file {}: {}", path.display(), e))
    }
}
