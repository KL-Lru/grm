use std::path::PathBuf;
pub use std::path::absolute;

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
