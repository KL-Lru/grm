use std::path::PathBuf;

use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::{build_shared_path, is_symlink};

/// Execute the worktree unshare command
///
/// Removes sharing of a file or directory between worktrees.
/// Removes symbolic links pointing to the shared file.
///
/// # Arguments
/// * `path_str` - Relative path to the shared file/directory
pub fn execute(path_str: &str) -> Result<(), GrmError> {
    let relative_path = PathBuf::from(path_str);

    // 1. Validate inside managed repo
    let repo_root = git::get_repo_root().map_err(|_| GrmError::NotInManagedRepository)?;
    let remote_url =
        git::get_remote_url(&repo_root).map_err(|_| GrmError::NotInManagedRepository)?;
    let repo_info = parse_git_url(&remote_url)?;

    // 2. Compute shared path
    let config = Config::load()?;
    let shared_path = build_shared_path(config.root(), &repo_info, &relative_path);

    // Canonicalize shared path to ensure reliable comparison
    let canonical_shared_path = if shared_path.exists() {
        shared_path.canonicalize()?
    } else {
        // If shared path doesn't exist, maybe it was deleted manually?
        // We can still try to remove symlinks that point to it (broken links).
        // But we can't canonicalize a non-existent path easily without resolving symlinks.
        // Let's use the absolute path we constructed.
        // For comparison with symlink targets, we might need absolute path.
        shared_path.clone()
    };

    // 3. Iterate over all worktrees
    let worktrees = git::get_worktrees(&repo_root)?;

    // We also need the relative path from the *repository root* (not CWD)
    // to find the target in other worktrees.
    // Assuming path_str is relative to CWD.
    let current_dir = std::env::current_dir()?;
    let absolute_target_path = current_dir.join(&relative_path);

    let path_relative_to_root = absolute_target_path
        .strip_prefix(&repo_root)
        .map_err(|_| GrmError::NotInManagedRepository)?
        .to_path_buf();

    let mut removed_count = 0;

    for worktree in worktrees {
        let target_in_worktree = worktree.join(&path_relative_to_root);

        if !target_in_worktree.exists() && !is_symlink(&target_in_worktree) {
            continue;
        }

        if is_symlink(&target_in_worktree) {
            // Simplified check:
            let is_match = if let Ok(link_target) = std::fs::read_link(&target_in_worktree) {
                link_target == shared_path
                 || (link_target.is_relative() && target_in_worktree.parent().map(|p| p.join(&link_target)).unwrap_or(link_target) == shared_path)
                 // Also compare canonicals if both exist
                 || (shared_path.exists() && target_in_worktree.canonicalize().ok() == canonical_shared_path.canonicalize().ok())
            } else {
                false
            };

            if is_match {
                std::fs::remove_file(&target_in_worktree)?;

                removed_count += 1;
            }
        }
    }

    if removed_count > 0 {
        println!("Unshared {path_str} from {removed_count} worktrees");
    } else {
        // "If path is not shared, this command performs no operation."
        // We can just print nothing or a debug message.
        // README says "performs no operation" implies silent or minimal feedback.
    }

    Ok(())
}
