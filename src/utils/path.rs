pub use std::path::absolute;
use std::path::{Path, PathBuf};

use crate::configs::ConfigError;
use crate::utils::git_url::RepoInfo;

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

/// Build a repository path following the grm structure
///
/// Creates a path following the `<root>/<host>/<user>/<repo>+<branch>` pattern.
pub fn build_repo_path(root: &Path, info: &RepoInfo, branch: &str) -> PathBuf {
    root.join(&info.host)
        .join(&info.user)
        .join(format!("{}+{}", info.repo, branch))
}

/// Build a shared storage path for a repository
///
/// Creates a path following the `$(grm root)/.shared/<host>/<user>/<repo>/<relative_path>` pattern.
pub fn build_shared_path(root: &Path, info: &RepoInfo, relative_path: &Path) -> PathBuf {
    root.join(".shared")
        .join(&info.host)
        .join(&info.user)
        .join(&info.repo)
        .join(relative_path)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_git_repository() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("repo");
        std::fs::create_dir(&repo_dir).unwrap();

        // Initially not a git repo
        assert!(!is_git_repository(&repo_dir));

        // Create .git directory
        let git_dir = repo_dir.join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        assert!(is_git_repository(&repo_dir));
    }

    #[test]
    fn test_is_git_repository_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("worktree");
        std::fs::create_dir(&repo_dir).unwrap();

        // Create .git file (like in worktrees or submodules)
        let git_file = repo_dir.join(".git");
        std::fs::File::create(&git_file).unwrap();
        assert!(is_git_repository(&repo_dir));
    }

    #[test]
    fn test_is_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        std::fs::File::create(&target).unwrap();

        let link = temp_dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(is_symlink(&link));
        assert!(!is_symlink(&target));
    }

    #[test]
    fn test_normalize_path_absolute() {
        // Use a definitely absolute path
        let abs_path = "/tmp";

        let path = normalize_path(abs_path).expect("Should parse absolute path");
        assert!(path.is_absolute());
    }

    #[test]
    fn test_normalize_path_empty() {
        assert!(normalize_path("").is_err());
        assert!(normalize_path("   ").is_err());
    }

    #[test]
    fn test_build_shared_path() {
        let root = PathBuf::from("/home/user/grm");
        let info = RepoInfo {
            host: "github.com".to_string(),
            user: "test".to_string(),
            repo: "repo".to_string(),
        };
        let path = build_shared_path(&root, &info, Path::new(".env"));
        assert_eq!(
            path,
            PathBuf::from("/home/user/grm/.shared/github.com/test/repo/.env")
        );
    }
}
