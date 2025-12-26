//! Filesystem utilities.

use genesis_types::Result;
use std::path::{Path, PathBuf};
use std::fs;

/// Expand path with tilde and environment variables.
pub fn expand_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    // Handle tilde expansion
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }

    path.to_path_buf()
}

/// Read entire file as string (slurp).
pub fn slurp(path: impl AsRef<Path>) -> Result<String> {
    fs::read_to_string(path).map_err(Into::into)
}

/// Write string to file (overwrite).
pub fn write_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
    fs::write(path, content).map_err(Into::into)
}

/// Ensure directory exists, creating if necessary.
pub fn ensure_dir(path: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(path).map_err(Into::into)
}

/// Recursively copy directory.
pub fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    fs::create_dir_all(dst)?;

    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let path = entry.path();

        if let Ok(rel_path) = path.strip_prefix(src) {
            let dst_path = dst.join(rel_path);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&dst_path)?;
            } else {
                fs::copy(path, dst_path)?;
            }
        }
    }

    Ok(())
}

/// Make path relative to home directory using ~.
pub fn humanize_path(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();

    if let Some(home) = dirs::home_dir() {
        if let Ok(rel) = path.strip_prefix(&home) {
            return format!("~/{}", rel.display());
        }
    }

    path.display().to_string()
}

// Note: Temporary file/directory creation is provided by the tempfile crate
// which offers secure temporary file handling with automatic cleanup.
