use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    Trash,
    Permanent,
}

#[derive(Debug, Clone)]
pub enum DeleteProgressMessage {
    Progress {
        current_target: usize,
        total_targets: usize,
        completed_targets: usize,
        current_path: String,
        freed_bytes: u64,
        total_bytes: u64,
        mode: DeleteMode,
    },
    TargetFinished {
        path: PathBuf,
        success: bool,
    },
    Done {
        freed_bytes: u64,
        errors: usize,
        mode: DeleteMode,
        cancelled: bool,
    },
}

#[derive(Debug, Default)]
pub struct DeleteResult {
    pub success_count: usize,
    pub fail_count: usize,
    pub freed_bytes: u64,
    pub errors: Vec<(PathBuf, String)>,
}

/// Deletes an artifact directory using the chosen mode (Trash or Permanent).
pub fn delete_path(path: &Path, mode: DeleteMode) -> Result<(), String> {
    delete_path_with_progress(path, mode, &AtomicBool::new(false), |_| {})
}

/// Deletes an artifact directory using the chosen mode and reports incremental freed bytes.
pub fn delete_path_with_progress<F>(
    path: &Path,
    mode: DeleteMode,
    cancel: &AtomicBool,
    mut on_bytes_freed: F,
) -> Result<(), String>
where
    F: FnMut(u64) + Send,
{
    if cancel.load(Ordering::Relaxed) {
        return Err("Deletion cancelled".to_string());
    }

    if !path.exists() {
        return Ok(());
    }

    match mode {
        DeleteMode::Trash => {
            if cancel.load(Ordering::Relaxed) {
                return Err("Deletion cancelled".to_string());
            }
            trash::delete(path).map_err(|e| format!("Failed to move to Trash: {}", e))
        }
        DeleteMode::Permanent => {
            fast_permanent_remove_with_progress(path, &mut on_bytes_freed, cancel)
        }
    }
}

/// Fast permanent recursive removal.
/// On Windows, renames to a temporary folder in the same parent directory first
/// so the main project directory is immediately cleared, then removes the folder in parallel.
fn fast_permanent_remove_with_progress<F>(
    path: &Path,
    on_bytes_freed: &mut F,
    cancel: &AtomicBool,
) -> Result<(), String>
where
    F: FnMut(u64) + Send,
{
    if cancel.load(Ordering::Relaxed) {
        return Err("Deletion cancelled".to_string());
    }

    if let Some(parent) = path.parent() {
        let temp_name = format!(
            ".degunk_tmp_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let temp_path = parent.join(temp_name);

        // Try fast rename first
        if fs::rename(path, &temp_path).is_ok() {
            if let Err(e) = remove_dir_all_accelerated(&temp_path, on_bytes_freed, cancel) {
                // If deletion fails, attempt to restore the original path
                let _ = fs::rename(&temp_path, path);
                return Err(format!(
                    "Failed to delete directory {}: {}",
                    path.display(),
                    e
                ));
            }
            return Ok(());
        }
    }

    // Direct removal fallback
    if path.is_dir() {
        remove_dir_all_accelerated(path, on_bytes_freed, cancel)
            .map_err(|e| format!("Failed to delete directory {}: {}", path.display(), e))
    } else {
        if cancel.load(Ordering::Relaxed) {
            return Err("Deletion cancelled".to_string());
        }
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        remove_file_resilient(path)
            .map(|()| on_bytes_freed(size))
            .map_err(|e| format!("Failed to delete file {}: {}", path.display(), e))
    }
}

/// Deletes directory contents in parallel using Rayon and streams freed byte progress.
fn remove_dir_all_accelerated<F>(
    path: &Path,
    on_bytes_freed: &mut F,
    cancel: &AtomicBool,
) -> std::io::Result<()>
where
    F: FnMut(u64) + Send,
{
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    for entry in walkdir::WalkDir::new(path)
        .same_file_system(true)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Deletion cancelled",
            ));
        }
        let p = entry.path().to_path_buf();
        if p == path {
            continue;
        }
        if entry.file_type().is_dir() {
            dirs.push(p);
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            files.push((p, size));
        }
    }

    // Delete files in parallel chunks of 1000 and report incremental progress
    for chunk in files.chunks(1000) {
        if cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Deletion cancelled",
            ));
        }
        let chunk_freed = AtomicU64::new(0);

        chunk.par_iter().for_each(|(file_path, size)| {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            if remove_file_resilient(file_path).is_ok() {
                chunk_freed.fetch_add(*size, Ordering::Relaxed);
            }
        });

        let freed = chunk_freed.load(Ordering::Relaxed);
        if freed > 0 {
            on_bytes_freed(freed);
        }

        if cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Deletion cancelled",
            ));
        }
    }

    if cancel.load(Ordering::Relaxed) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "Deletion cancelled",
        ));
    }

    // Remove directories in bottom-up order (deepest first)
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for dir in dirs {
        if cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Deletion cancelled",
            ));
        }
        let _ = fs::remove_dir(&dir);
    }

    if cancel.load(Ordering::Relaxed) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "Deletion cancelled",
        ));
    }

    // Finally remove the target directory root
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Resilient fallback if any entries still remain
            remove_dir_all_resilient(path)
        }
    }
}

/// Recursively removes a directory, clearing read-only attributes if permissions fail.
fn remove_dir_all_resilient(path: &Path) -> std::io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            // Clear read-only attributes on all entries and retry
            for entry in walkdir::WalkDir::new(path)
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
            fs::remove_dir_all(path)
        }
        Err(e) => Err(e),
    }
}

/// Removes a file, clearing read-only attribute if permission is denied.
fn remove_file_resilient(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            if let Ok(metadata) = fs::metadata(path) {
                let mut perms = metadata.permissions();
                if perms.readonly() {
                    perms.set_readonly(false);
                    let _ = fs::set_permissions(path, perms);
                }
            }
            fs::remove_file(path)
        }
        Err(e) => Err(e),
    }
}
