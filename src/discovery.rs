use std::path::{Path, PathBuf};

use crate::utils::git_url::RepoInfo;
use crate::utils::path::{is_git_repository, is_symlink};

/// Scan for git repositories under the root directory
///
/// Searches for repositories following the `<host>/<user>/<repo>+<branch>` directory structure.
/// Only directories containing a `.git` directory are considered valid repositories.
///
/// # Arguments
/// * `root` - Root directory to scan
///
/// # Returns
/// * `Vec<PathBuf>` - List of absolute paths to discovered repositories
pub fn scan_repositories(root: &Path) -> Vec<PathBuf> {
    let host_dirs: Vec<PathBuf> = read_valid_directories(root).collect();

    let user_dirs: Vec<PathBuf> = host_dirs
        .iter()
        .flat_map(|host_path| read_valid_directories(host_path))
        .collect();

    let repositories: Vec<PathBuf> = user_dirs
        .iter()
        .flat_map(|user_path| read_valid_directories(user_path))
        .filter(|repo_path| is_valid_repo_pattern(repo_path) && is_git_repository(repo_path))
        .collect();

    repositories
}

/// Find all repositories matching the given repository info
///
/// Searches for directories matching `<root>/<host>/<user>/<repo>+*` pattern.
/// Only returns directories that contain a `.git` directory.
///
/// # Arguments
/// * `root` - Root directory to search in
/// * `info` - Parsed repository information
///
/// # Returns
/// * `Vec<PathBuf>` - List of matching repository paths
pub fn find_matching_repositories(root: &Path, info: &RepoInfo) -> Vec<PathBuf> {
    let target_path = root.join(&info.host).join(&info.user);

    if !target_path.exists() {
        return Vec::new();
    }

    let prefix = format!("{}+", info.repo);
    // Use read_valid_directories to handle safe directory reading
    read_valid_directories(&target_path)
        .filter(|path| {
            // Check if directory name starts with "<repo>+"
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .filter(|path| {
            // Check if it's a git repository
            is_git_repository(path)
        })
        .collect()
}

/// Read directories from a path, filtering out symlinks and non-directories
///
/// Returns an iterator of valid directory paths, skipping any entries that:
/// - Are symlinks
/// - Are not directories
/// - Cannot be read due to permissions or other IO errors
pub fn read_valid_directories(path: &Path) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|p| !is_symlink(p) && p.is_dir())
}

/// Check if a directory name matches the `<repo>+<branch>` pattern
///
/// Valid patterns must have:
/// - Exactly one '+' character
/// - Non-empty repository name before '+'
/// - Non-empty branch name after '+'
///
/// # Examples
/// - `repo+main` -> true
/// - `my-repo+feature/awesome` -> true
/// - `+branch` -> false (empty repo)
/// - `repo+` -> false (empty branch)
/// - `repo+branch+extra` -> false (multiple '+')
fn is_valid_repo_pattern(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name_str| {
            let plus_pos = name_str.find('+')?;
            let repo_part = &name_str[..plus_pos];
            let branch_part = &name_str[plus_pos + 1..];

            // Check: repo not empty, branch not empty, no additional '+'
            if !repo_part.is_empty() && !branch_part.is_empty() && !branch_part.contains('+') {
                Some(())
            } else {
                None
            }
        })
        .is_some()
}
