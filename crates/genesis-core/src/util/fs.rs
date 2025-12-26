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

// TODO: Implement:
// - Temporary file/directory creation
// - Safe file operations
// - Path humanization
// - Directory walking
// - Copy operations
