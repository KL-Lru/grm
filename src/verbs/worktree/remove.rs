use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::build_repo_path;

/// Execute the worktree remove command
///
/// Removes the worktree for the specified branch.
///
/// # Arguments
/// * `branch` - Branch name of the worktree to remove
///
/// # Returns
/// * `Ok(())` on success
/// * `Err` if not in a managed repository, or if worktree removal fails
pub fn execute(branch: &str) -> Result<(), GrmError> {
    // Get repository root from current directory
    let repo_root = git::get_repo_root().map_err(|_| GrmError::NotInManagedRepository)?;

    // Get remote URL
    let remote_url =
        git::get_remote_url(&repo_root).map_err(|_| GrmError::NotInManagedRepository)?;

    // Parse URL to get repo info
    let repo_info = parse_git_url(&remote_url)?;

    // Load config to get root directory
    let config = Config::load()?;
    let worktree_path = build_repo_path(config.root(), &repo_info, branch);

    if !worktree_path.exists() {
        return Err(GrmError::NotFound(format!(
            "Worktree does not exist: {}",
            worktree_path.display()
        )));
    }

    git::remove_worktree(&repo_root, &worktree_path).map_err(GrmError::Git)?;

    println!("Removed worktree: {}", worktree_path.display());

    Ok(())
}
