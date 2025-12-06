use std::path::PathBuf;

use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::parse_git_url;
use crate::utils::path::{build_shared_path, is_symlink};

/// Execute the worktree share command
///
/// Shares a file or directory between all worktrees of a repository.
///
/// # Arguments
/// * `path_str` - Relative path to the file/directory to share
pub fn execute(path_str: &str) -> Result<(), GrmError> {
    let relative_path = PathBuf::from(path_str);

    // 1. Validate inside managed repo
    let repo_root = git::get_repo_root().map_err(|_| GrmError::NotInManagedRepository)?;
    let remote_url =
        git::get_remote_url(&repo_root).map_err(|_| GrmError::NotInManagedRepository)?;
    let repo_info = parse_git_url(&remote_url)?;

    let current_dir = std::env::current_dir()?;
    let absolute_source_path = current_dir.join(&relative_path);

    if !absolute_source_path.exists() {
        return Err(GrmError::NotFound(format!(
            "File/Directory not found: {path_str}"
        )));
    }

    let config = Config::load()?;
    let shared_path = build_shared_path(config.root(), &repo_info, &relative_path);

    let worktrees = git::get_worktrees(&repo_root)?;
    let path_relative_to_root = absolute_source_path
        .strip_prefix(&repo_root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut conflicts = Vec::new();
    for worktree in &worktrees {
        let target_in_worktree = worktree.join(path_relative_to_root);

        if let Ok(canon_target) = target_in_worktree.canonicalize()
            && let Ok(canon_source) = absolute_source_path.canonicalize()
            && canon_target == canon_source
        {
            continue;
        }

        if target_in_worktree.exists() && !is_symlink(&target_in_worktree) {
            conflicts.push(target_in_worktree);
        }
    }

    if !conflicts.is_empty() {
        println!("The following files will be overwritten:");
        for conflict in &conflicts {
            println!("  {}", conflict.display());
        }
        println!("Do you want to continue? [y/N]");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            return Err(GrmError::UserCancelled);
        }
    }

    if is_symlink(&absolute_source_path) {
        if absolute_source_path.canonicalize().ok() == shared_path.canonicalize().ok() {
            println!("{path_str} is already shared.");
        }
    } else {
        if let Some(parent) = shared_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if shared_path.exists() {
            if shared_path.is_dir() {
                std::fs::remove_dir_all(&shared_path)?;
            } else {
                std::fs::remove_file(&shared_path).ok();
            }
        }

        std::fs::rename(&absolute_source_path, &shared_path)?;
    }

    let count = worktrees.len();
    for worktree in worktrees {
        let target_in_worktree = worktree.join(path_relative_to_root);

        if target_in_worktree.exists() || is_symlink(&target_in_worktree) {
            if target_in_worktree.is_dir() && !is_symlink(&target_in_worktree) {
                std::fs::remove_dir_all(&target_in_worktree)?;
            } else {
                std::fs::remove_file(&target_in_worktree)?;
            }
        }

        std::os::unix::fs::symlink(&shared_path, &target_in_worktree)?;
    }

    println!("Shared {path_str} across {count} worktrees");
    Ok(())
}
