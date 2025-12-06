use std::path::{Path, PathBuf};

use crate::{
    configs::Config,
    utils::path::{is_git_repository, is_symlink},
};

/// Execute the list command
///
/// Lists all managed repositories under the configured root directory.
/// Repositories are displayed in alphabetical order.
///
/// # Arguments
/// * `full_path` - If true, shows absolute paths; otherwise shows relative paths from root
///
/// # Returns
/// * `Ok(())` on success
/// * `Err` if configuration loading or directory scanning fails
pub fn execute(full_path: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let root = config.root();

    // If root doesn't exist, print "Nothing to display" and exit normally
    if !root.exists() {
        println!("Nothing to display");
        return Ok(());
    }

    // Scan for repositories
    let mut repositories = scan_repositories(root)?;

    // If no repositories found, print "Nothing to display"
    if repositories.is_empty() {
        println!("Nothing to display");
        return Ok(());
    }

    // Sort alphabetically
    repositories.sort();

    // Print repositories
    for repo in repositories {
        if full_path {
            println!("{}", repo.display());
        } else {
            // Show relative path from root
            match repo.strip_prefix(root) {
                Ok(relative) => println!("{}", relative.display()),
                Err(_) => println!("{}", repo.display()),
            }
        }
    }

    Ok(())
}

/// Scan for git repositories under the root directory
///
/// Searches for repositories following the `<host>/<user>/<repo>+<branch>` directory structure.
/// Only directories containing a `.git` directory are considered valid repositories.
///
/// # Arguments
/// * `root` - Root directory to scan
///
/// # Returns
/// * `Ok(Vec<PathBuf>)` - List of absolute paths to discovered repositories
/// * `Err` - IO error (currently returns empty list on error)
fn scan_repositories(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
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

    Ok(repositories)
}

/// Read directories from a path, filtering out symlinks and non-directories
///
/// Returns an iterator of valid directory paths, skipping any entries that:
/// - Are symlinks
/// - Are not directories
/// - Cannot be read due to permissions or other IO errors
fn read_valid_directories(path: &Path) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
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
