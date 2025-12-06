pub use std::path::absolute;
use std::path::{Path, PathBuf};

use crate::configs::ConfigError;

/// Get the home directory path
///
/// Uses the `dirs` crate for cross-platform home directory detection.
pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir().and_then(|path| absolute(&path).ok())
}

/// Get the home directory path or return an error
///
/// This is a convenience wrapper around `home_dir()` that returns
/// a `ConfigError` instead of `None`.
pub fn require_home_dir() -> Result<PathBuf, ConfigError> {
    home_dir().ok_or_else(|| ConfigError::Path("Home directory not found".into()))
}

/// Check if a directory is a git repository
///
/// A directory is considered a git repository if it contains a `.git` directory or file.
/// The `.git` can be either a directory (normal repository) or a file (submodule/worktree).
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// * `true` if the path contains a `.git` directory or file
/// * `false` otherwise
pub fn is_git_repository(path: &Path) -> bool {
    let git_path = path.join(".git");
    git_path.exists() && (git_path.is_dir() || git_path.is_file())
}

/// Check if a path is a symlink
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// * `true` if the path is a symbolic link
/// * `false` if not a symlink or if metadata cannot be read
pub fn is_symlink(path: &Path) -> bool {
    match path.symlink_metadata() {
        Ok(metadata) => metadata.is_symlink(),
        Err(_) => false,
    }
}

/// Normalize a path string to an absolute ``PathBuf``
///
/// # Path Resolution Rules
///
/// - `~` or `~/path`: Expanded to home directory
/// - `/absolute/path`: Used as-is (absolute path)
/// - `relative/path`: **Resolved from HOME directory** (not current working directory)
///
/// **Important**: Relative paths are resolved relative to the home directory,
/// not the current working directory. This behavior is intentional for
/// configuration files to ensure consistent paths regardless of where
/// the command is run from.
///
/// # Examples
///
/// ```ignore
/// normalize_path("~/grm")?;          // -> /home/user/grm
/// normalize_path("~")?;              // -> /home/user
/// normalize_path("/tmp/grm")?;       // -> /tmp/grm (absolute, unchanged)
/// normalize_path("relative/path")?;  // -> /home/user/relative/path (NOT $PWD/relative/path)
/// normalize_path("  ~/grm  ")?;      // -> /home/user/grm (trimmed)
/// ```
///
/// # Errors
///
/// Returns `ConfigError::PathError` if:
/// - The path is empty after trimming
/// - Home directory cannot be found
/// - Path starts with `~` but is not `~` or `~/...` (e.g., `~user/path`)
/// - Path cannot be converted to an absolute path
pub fn normalize_path(path_str: &str) -> Result<PathBuf, ConfigError> {
    let path_str = path_str.trim();

    if path_str.is_empty() {
        return Err(ConfigError::Path("Empty path".into()));
    }

    let path = if path_str.starts_with('~') {
        let home = require_home_dir()?;

        if path_str.len() == 1 {
            home
        } else if let Some(subpath) = path_str.strip_prefix("~/") {
            if subpath.is_empty() {
                home
            } else {
                home.join(subpath)
            }
        } else {
            return Err(ConfigError::Path(format!(
                "Path '{path_str}' is not supported. Use absolute path or ~/path format. User-specific paths (~user/) are not yet supported.",
            )));
        }
    } else {
        let path = PathBuf::from(path_str);
        if path.is_absolute() {
            path
        } else {
            // Relative paths are resolved relative to home directory
            let home = require_home_dir()?;
            home.join(path)
        }
    };

    // Convert to absolute path
    absolute(&path).map_err(|e| ConfigError::Path(e.to_string()))
}
