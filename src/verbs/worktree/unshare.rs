use std::path::PathBuf;

use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::{build_shared_path, is_symlink};

/// Execute the worktree unshare command
///
/// Removes sharing of a file or directory between worktrees.
pub fn execute(path_str: &str) -> Result<(), GrmError> {
    let relative_path = PathBuf::from(path_str);

    let repo_root = git::get_repo_root().map_err(|_| GrmError::NotInManagedRepository)?;
    let remote_url =
        git::get_remote_url(&repo_root).map_err(|_| GrmError::NotInManagedRepository)?;
    let repo_info = parse_git_url(&remote_url)?;

    let config = Config::load()?;
    let shared_path = build_shared_path(config.root(), &repo_info, &relative_path);

    let canonical_shared_path = if shared_path.exists() {
        shared_path.canonicalize()?
    } else {
        shared_path.clone()
    };

    let worktrees = git::get_worktrees(&repo_root)?;

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
            let is_match = if let Ok(link_target) = std::fs::read_link(&target_in_worktree) {
                link_target == shared_path
                 || (link_target.is_relative() && target_in_worktree.parent().map(|p| p.join(&link_target)).unwrap_or(link_target) == shared_path)
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
    }

    Ok(())
}
