use std::path::{Path, PathBuf};

use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git_url::{RepoInfo, parse_git_url};
use crate::utils::prompt;

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
/// * `Ok(Vec<PathBuf>)` - List of matching repository paths
/// * `Err` - IO error during directory scanning
fn find_matching_repositories(
    root: &Path,
    info: &RepoInfo,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let target_path = root.join(&info.host).join(&info.user);

    if !target_path.exists() {
        return Ok(Vec::new());
    }

    let prefix = format!("{}+", info.repo);
    let matching_repos: Vec<PathBuf> = std::fs::read_dir(&target_path)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            // Check if it's a directory and not a symlink
            !is_symlink(path) && path.is_dir()
        })
        .filter(|path| {
            // Check if directory name starts with "<repo>+"
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&prefix))
                .unwrap_or(false)
        })
        .filter(|path| {
            // Check if it's a git repository
            is_git_repository(path)
        })
        .collect();

    Ok(matching_repos)
}

/// Prompt user for confirmation before deletion
///
/// Displays the list of repositories to be deleted and asks for confirmation.
/// If `force` is true, skips the prompt and returns true.
///
/// # Arguments
/// * `repositories` - List of repositories to delete
/// * `force` - If true, skip confirmation prompt
///
/// # Returns
/// * `Ok(true)` if deletion should proceed
/// * `Ok(false)` if user cancelled
/// * `Err` - IO error during prompt
fn prompt_confirmation(repositories: &[PathBuf], force: bool) -> Result<bool, GrmError> {
    if force {
        return Ok(true);
    }

    println!("The following repositories will be deleted:");
    for repo in repositories {
        println!("  - {}", repo.display());
    }
    println!();

    Ok(prompt::confirm("Do you want to continue?")?)
}

/// Delete the given repositories
///
/// Removes each repository directory recursively.
/// Skips any paths that are symbolic links for safety.
///
/// # Arguments
/// * `repositories` - List of repository paths to delete
///
/// # Returns
/// * `Ok(())` on success
/// * `Err` if deletion fails for any repository
///
/// # Safety
/// This function uses `remove_dir_all` which recursively deletes directories.
/// Symlink check is performed as a safety measure, though repositories
/// should already be filtered by `find_matching_repositories`.
fn remove_repositories(repositories: &[PathBuf]) -> Result<(), GrmError> {
    for repo in repositories {
        // Safety check: skip if it's a symlink (defensive programming)
        if is_symlink(repo) {
            eprintln!(
                "Warning: Skipping symlink: {} (unexpected, should have been filtered)",
                repo.display()
            );
            continue;
        }

        std::fs::remove_dir_all(repo)?;
        println!("Removed: {}", repo.display());
    }
    Ok(())
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
fn is_git_repository(path: &Path) -> bool {
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
fn is_symlink(path: &Path) -> bool {
    match path.symlink_metadata() {
        Ok(metadata) => metadata.is_symlink(),
        Err(_) => false,
    }
}

/// Execute the remove command
///
/// Removes all branches of a repository matching the given URL.
///
/// # Arguments
/// * `url` - Git repository URL
/// * `force` - If true, skip confirmation prompt
///
/// # Returns
/// * `Ok(())` on success
/// * `Err` if URL is invalid, repository not found, or deletion fails
pub fn execute(url: &str, force: bool) -> Result<(), GrmError> {
    // Parse the URL
    let repo_info = parse_git_url(url)?;

    // Load configuration to get root directory
    let config = Config::load()?;
    let root = config.root();

    // Find matching repositories
    let matching_repos = find_matching_repositories(root, &repo_info)?;

    if matching_repos.is_empty() {
        let searched_path = root.join(&repo_info.host).join(&repo_info.user);
        return Err(GrmError::UnmanagedRepository {
            url: url.to_string(),
            searched_path: searched_path.display().to_string(),
        });
    }

    // Prompt for confirmation (unless --force)
    if !prompt_confirmation(&matching_repos, force)? {
        return Err(GrmError::UserCancelled);
    }

    // Remove repositories
    remove_repositories(&matching_repos)?;

    println!(
        "\nSuccessfully removed {} repository(ies).",
        matching_repos.len()
    );

    Ok(())
}
