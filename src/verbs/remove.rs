use std::path::PathBuf;

use crate::configs::Config;
use crate::discovery::find_matching_repositories;
use crate::errors::GrmError;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::is_symlink;
use crate::utils::prompt;

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

    prompt::confirm("Do you want to continue?")
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
    let repo_info = parse_git_url(url)?;
    let config = Config::load()?;
    let root = config.root();

    let matching_repos = find_matching_repositories(root, &repo_info);

    if matching_repos.is_empty() {
        let searched_path = root.join(&repo_info.host).join(&repo_info.user);
        return Err(GrmError::UnmanagedRepository {
            url: url.to_string(),
            searched_path: searched_path.display().to_string(),
        });
    }

    if !prompt_confirmation(&matching_repos, force)? {
        return Err(GrmError::UserCancelled);
    }

    remove_repositories(&matching_repos)?;

    println!(
        "\nSuccessfully removed {} repository(ies).",
        matching_repos.len()
    );

    Ok(())
}
