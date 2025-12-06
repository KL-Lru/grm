use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::build_repo_path;

/// Execute the worktree split command
///
/// Creates a new worktree for the specified branch.
///
/// # Arguments
/// * `branch` - Branch name to create worktree for
///
/// # Returns
/// * `Ok(())` on success, prints the created worktree path
/// * `Err` if not in a managed repository, path already exists, or git command fails
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
    let dest_path = build_repo_path(config.root(), &repo_info, branch);

    // Check if worktree path already exists
    if dest_path.exists() {
        return Err(GrmError::AlreadyExists(dest_path.display().to_string()));
    }

    // Create parent directories (handles nested dirs for branches with slashes)
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Determine branch priority: local -> remote -> new
    if git::local_branch_exists(branch)? {
        // Use existing local branch
        git::add_worktree(&dest_path, branch, false)?;
    } else if git::remote_branch_exists(&remote_url, branch)? {
        // Checkout from remote branch
        git::add_worktree(&dest_path, branch, false)?;
    } else {
        // Create new branch
        git::add_worktree(&dest_path, branch, true)?;
    }

    println!("{}", dest_path.display());

    Ok(())
}
