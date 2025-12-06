use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::build_repo_path;

/// Execute the worktree remove command
///
/// Removes the worktree for the specified branch.
pub fn execute(branch: &str) -> Result<(), GrmError> {
    let repo_root = git::get_repo_root().map_err(|_| GrmError::NotInManagedRepository)?;
    let remote_url =
        git::get_remote_url(&repo_root).map_err(|_| GrmError::NotInManagedRepository)?;
    let repo_info = parse_git_url(&remote_url)?;

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
