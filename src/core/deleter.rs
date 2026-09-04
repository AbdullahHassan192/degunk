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
        freed_bytes: u64,
    },
    Done {
        freed_bytes: u64,
        errors: usize,
        mode: DeleteMode,
        cancelled: bool,
        error_details: Vec<String>,
        log_path: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DeleteResult {
    pub success_count: usize,
    pub fail_count: usize,
    pub freed_bytes: u64,
    pub errors: Vec<(PathBuf, String)>,
}

#[derive(Debug, Clone)]
pub struct DeletionTargetError {
    pub target_path: PathBuf,
    pub message: String,
    pub file_errors: Vec<(PathBuf, String)>,
}

impl DeletionTargetError {
    pub fn new(target_path: PathBuf, message: impl Into<String>) -> Self {
        Self {
            target_path,
            message: message.into(),
            file_errors: Vec::new(),
        }
    }

    pub fn with_file_errors(
        target_path: PathBuf,
        message: impl Into<String>,
        file_errors: Vec<(PathBuf, String)>,
    ) -> Self {
        Self {
            target_path,
            message: message.into(),
            file_errors,
        }
    }
}

impl std::fmt::Display for DeletionTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.file_errors.is_empty() {
            write!(f, "{}: {}", self.target_path.display(), self.message)
        } else {
            let files_summary: Vec<String> = self
                .file_errors
                .iter()
                .take(3)
                .map(|(p, e)| format!("{}: {}", p.display(), e))
                .collect();
            let more = if self.file_errors.len() > 3 {
                format!(" (and {} more)", self.file_errors.len() - 3)
            } else {
                String::new()
            };
            write!(
                f,
                "{}: {} [failed files: {}{}]",
                self.target_path.display(),
                self.message,
                files_summary.join("; "),
                more
            )
        }
    }
}

impl std::error::Error for DeletionTargetError {}

/// Returns the path to the persistent degunk deletion error log.
pub fn get_error_log_path() -> PathBuf {
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("degunk");
        let _ = fs::create_dir_all(&dir);
        dir.join("degunk_errors.log")
    } else if let Some(mut home) = dirs::home_dir() {
        home.push(".degunk");
        let _ = fs::create_dir_all(&home);
        home.join("degunk_errors.log")
    } else {
        std::env::temp_dir().join("degunk_errors.log")
    }
}

/// Appends a structured error report to the degunk error log file.
pub fn log_deletion_errors(errors: &[DeletionTargetError]) {
    if errors.is_empty() {
        return;
    }
    let log_file = get_error_log_path();
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&log_file) {
        use std::io::Write;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "==================================================");
        let _ = writeln!(
            file,
            "[{}] Deletion Error Report ({} target(s) failed)",
            now,
            errors.len()
        );
        for err in errors {
            let _ = writeln!(file, "Target: {}", err.target_path.display());
            let _ = writeln!(file, "Reason: {}", err.message);
            if !err.file_errors.is_empty() {
                let _ = writeln!(file, "Failed items inside target:");
                for (fp, msg) in &err.file_errors {
                    let _ = writeln!(file, "  • {}: {}", fp.display(), msg);
                }
            }
            let _ = writeln!(file);
        }
    }
}

/// Proactively and recursively strips read-only attributes from files and directories.
/// This prevents Access Denied failures on Windows when moving to Recycle Bin or deleting.
pub fn strip_readonly_recursive(path: &Path, cancel: &AtomicBool) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let strip_entry = |p: &Path| {
        if let Ok(metadata) = fs::metadata(p) {
            let mut perms = metadata.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = fs::set_permissions(p, perms);
            }
        }
    };

    if !path.is_dir() {
        strip_entry(path);
        return Ok(());
    }

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if cancel.load(Ordering::Relaxed) {
            return Err("Deletion cancelled".to_string());
        }
        if let Ok(metadata) = entry.metadata() {
            let mut perms = metadata.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = fs::set_permissions(entry.path(), perms);
            }
        }
    }

    Ok(())
}

/// Scans a directory to detect files that are locked by active processes or cannot be accessed.
pub fn find_locked_or_inaccessible_files(path: &Path) -> Vec<(PathBuf, String)> {
    let mut locked = Vec::new();
    if !path.exists() {
        return locked;
    }

    let check_file = |p: &Path| -> Option<String> {
        #[cfg(windows)]
        {
            match fs::OpenOptions::new().write(true).open(p) {
                Ok(_) => None,
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("used by another process")
                        || err_str.contains("Access is denied")
                        || e.raw_os_error() == Some(32)
                        || e.raw_os_error() == Some(5)
                    {
                        Some(err_str)
                    } else {
                        None
                    }
                }
            }
        }
        #[cfg(not(windows))]
        {
            match fs::OpenOptions::new().write(true).open(p) {
                Ok(_) => None,
                Err(e) => Some(e.to_string()),
            }
        }
    };

    if path.is_file() {
        if let Some(err) = check_file(path) {
            locked.push((path.to_path_buf(), err));
        }
        return locked;
    }

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(err) = check_file(entry.path()) {
                locked.push((entry.path().to_path_buf(), err));
                if locked.len() >= 20 {
                    break;
                }
            }
        }
    }

    locked
}

/// Deletes an artifact directory using the chosen mode (Trash or Permanent).
pub fn delete_path(path: &Path, mode: DeleteMode) -> Result<(), DeletionTargetError> {
    delete_path_with_progress(path, mode, &AtomicBool::new(false), |_| {})
}

/// Deletes an artifact directory using the chosen mode and reports incremental freed bytes.
pub fn delete_path_with_progress<F>(
    path: &Path,
    mode: DeleteMode,
    cancel: &AtomicBool,
    mut on_bytes_freed: F,
) -> Result<(), DeletionTargetError>
where
    F: FnMut(u64) + Send,
{
    if cancel.load(Ordering::Relaxed) {
        return Err(DeletionTargetError::new(path.to_path_buf(), "Deletion cancelled"));
    }

    if !path.exists() {
        return Ok(());
    }

    match mode {
        DeleteMode::Trash => {
            if cancel.load(Ordering::Relaxed) {
                return Err(DeletionTargetError::new(path.to_path_buf(), "Deletion cancelled"));
            }
            // Strip read-only attributes before trashing to avoid Access Denied on Windows
            if let Err(e) = strip_readonly_recursive(path, cancel) {
                return Err(DeletionTargetError::new(path.to_path_buf(), e));
            }
            if cancel.load(Ordering::Relaxed) {
                return Err(DeletionTargetError::new(path.to_path_buf(), "Deletion cancelled"));
            }
            match trash::delete(path) {
                Ok(()) => Ok(()),
                Err(e) => {
                    let locked = find_locked_or_inaccessible_files(path);
                    Err(DeletionTargetError::with_file_errors(
                        path.to_path_buf(),
                        format!("Failed to move to Trash: {}", e),
                        locked,
                    ))
                }
            }
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
) -> Result<(), DeletionTargetError>
where
    F: FnMut(u64) + Send,
{
    if cancel.load(Ordering::Relaxed) {
        return Err(DeletionTargetError::new(path.to_path_buf(), "Deletion cancelled"));
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
            if let Err(mut file_errs) = remove_dir_all_accelerated(&temp_path, on_bytes_freed, cancel) {
                // If deletion fails, attempt to restore the original path
                let _ = fs::rename(&temp_path, path);
                // Map temp_path paths back to original path
                for (err_path, _) in &mut file_errs {
                    if let Ok(rel) = err_path.strip_prefix(&temp_path) {
                        *err_path = path.join(rel);
                    }
                }
                return Err(DeletionTargetError::with_file_errors(
                    path.to_path_buf(),
                    "Failed to delete directory (some files could not be removed)",
                    file_errs,
                ));
            }
            return Ok(());
        }
    }

    // Direct removal fallback
    if path.is_dir() {
        remove_dir_all_accelerated(path, on_bytes_freed, cancel)
            .map_err(|file_errs| {
                DeletionTargetError::with_file_errors(
                    path.to_path_buf(),
                    "Failed to delete directory (some files could not be removed)",
                    file_errs,
                )
            })
    } else {
        if cancel.load(Ordering::Relaxed) {
            return Err(DeletionTargetError::new(path.to_path_buf(), "Deletion cancelled"));
        }
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        remove_file_resilient(path)
            .map(|()| on_bytes_freed(size))
            .map_err(|e| {
                DeletionTargetError::with_file_errors(
                    path.to_path_buf(),
                    "Failed to delete file",
                    vec![(path.to_path_buf(), e.to_string())],
                )
            })
    }
}

/// Deletes directory contents in parallel using Rayon and streams freed byte progress.
fn remove_dir_all_accelerated<F>(
    path: &Path,
    on_bytes_freed: &mut F,
    cancel: &AtomicBool,
) -> Result<(), Vec<(PathBuf, String)>>
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
            return Err(vec![(path.to_path_buf(), "Deletion cancelled".to_string())]);
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

    let file_errors = std::sync::Mutex::new(Vec::new());

    // Delete files in parallel chunks of 1000 and report incremental progress
    for chunk in files.chunks(1000) {
        if cancel.load(Ordering::Relaxed) {
            return Err(vec![(path.to_path_buf(), "Deletion cancelled".to_string())]);
        }
        let chunk_freed = AtomicU64::new(0);

        chunk.par_iter().for_each(|(file_path, size)| {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            match remove_file_resilient(file_path) {
                Ok(()) => {
                    chunk_freed.fetch_add(*size, Ordering::Relaxed);
                }
                Err(e) => {
                    if let Ok(mut errs) = file_errors.lock() {
                        errs.push((file_path.clone(), e.to_string()));
                    }
                }
            }
        });

        let freed = chunk_freed.load(Ordering::Relaxed);
        if freed > 0 {
            on_bytes_freed(freed);
        }

        if cancel.load(Ordering::Relaxed) {
            return Err(vec![(path.to_path_buf(), "Deletion cancelled".to_string())]);
        }
    }

    if cancel.load(Ordering::Relaxed) {
        return Err(vec![(path.to_path_buf(), "Deletion cancelled".to_string())]);
    }

    // Remove directories in bottom-up order (deepest first)
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for dir in dirs {
        if cancel.load(Ordering::Relaxed) {
            return Err(vec![(path.to_path_buf(), "Deletion cancelled".to_string())]);
        }
        if let Err(e) = fs::remove_dir(&dir) {
            if let Ok(mut errs) = file_errors.lock() {
                errs.push((dir, e.to_string()));
            }
        }
    }

    if cancel.load(Ordering::Relaxed) {
        return Err(vec![(path.to_path_buf(), "Deletion cancelled".to_string())]);
    }

    // Finally remove the target directory root
    if let Err(e) = fs::remove_dir(path) {
        if let Ok(mut errs) = file_errors.lock() {
            errs.push((path.to_path_buf(), e.to_string()));
        }
    }

    let errs = file_errors.into_inner().unwrap_or_default();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

/// Attempts to remove a file with resiliency measures:
/// Strips the read-only attribute if needed and retries once on Access Denied.
fn remove_file_resilient(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) => {
            // Windows error code 5 is Access is Denied (frequently caused by read-only git pack files)
            if err.kind() == std::io::ErrorKind::PermissionDenied || err.raw_os_error() == Some(5) {
                if let Ok(metadata) = fs::metadata(path) {
                    let mut perms = metadata.permissions();
                    perms.set_readonly(false);
                    let _ = fs::set_permissions(path, perms);
                }
                fs::remove_file(path)
            } else {
                Err(err)
            }
        }
    }
}
