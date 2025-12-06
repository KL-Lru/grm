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

    // 2. Check source file existence (relative to CWD, which should be in a worktree)
    // We need to resolve path_str relative to current directory to get absolute path
    // then check if it is inside the current worktree.
    // However, the spec implies we run this inside a managed repository directory.
    // Let's assume path_str is relative to current directory.
    let current_dir = std::env::current_dir()?;
    let absolute_source_path = current_dir.join(&relative_path);

    if !absolute_source_path.exists() {
        return Err(GrmError::NotFound(format!(
            "File/Directory not found: {path_str}"
        )));
    }

    // 3. Compute shared path
    let config = Config::load()?;
    let shared_path = build_shared_path(config.root(), &repo_info, &relative_path);

    // 4. Check for conflicts in other worktrees
    let worktrees = git::get_worktrees(&repo_root)?;
    let path_relative_to_root = absolute_source_path
        .strip_prefix(&repo_root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut conflicts = Vec::new();
    for worktree in &worktrees {
        // Skip checking if it overlaps with source path?
        // Actually, source path IS in one of the worktrees.
        // We shouldn't count the source file as a "conflict" that needs overwriting (we are moving it).
        let target_in_worktree = worktree.join(path_relative_to_root);

        // Canonicalize to compare equality properly?
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

    // 5. Move original file to shared_path (if it's not already a symlink to there)
    // If it's already a symlink, we might just want to ensure it points to the right place?
    // Or maybe we act idempotently.
    // If it is NOT a symlink, we move it to shared path.
    if is_symlink(&absolute_source_path) {
        // If already a symlink, check where it points?
        // For now, let's assume if it is a symlink, it might be already shared.
        // But the user might want to share a NEW file, or Convert existing.
        // If it is a symlink, maybe we should warn or check?
        // Spec says: "If path is not in a managed repository, this command will fail."
        // If path is already shared, maybe we just update other worktrees.
        // Let's proceed to ensure shared_path exists.
        // If source is a symlink and shared_path doesn't exist, that's weird.
        // If source is a symlink, we probably shouldn't move it. We should assume the content is elsewhere.
        // If the user runs `share file` and `file` is a symlink, do we follow it?
        // Let's assume we want to resolve it if it's not pointing to our shared storage.
        // But simplest implementation: If it's a symlink, assume it's already handled or fail?
        // Spec doesn't detail this.
        // Let's assume standard flow: valid file/dir -> move to shared -> symlink back.
        // If it is a symlink, we might fail or Skip moving.
        if absolute_source_path.canonicalize().ok() == shared_path.canonicalize().ok() {
            println!("{path_str} is already shared.");
        } else {
            // It's a symlink to somewhere else?
            // We could copy content to shared and re-link.
            // For safety, let's treat symlinks as "already shared" or "special" and skip move.
        }
    } else {
        // Not a symlink, real file/dir. Move it.
        // Ensure shared parent dir exists
        if let Some(parent) = shared_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // If shared path already exists (e.g. from another worktree check in?), overwrite or fail?
        // Spec says "This operation **overwrites** the file/directory in each worktree."
        // Doesn't explicitly say what happens to the shared storage if it conflicts.
        // But likely we want to initialize shared storage from THIS worktree's version.
        // If shared path exists, we might be overwriting it with current version.
        if shared_path.exists() {
            // Backup? Or overwrite?
            // "This operation **overwrites** the file/directory in each worktree" implies aggressive.
            // Let's move current to shared, replacing whatever is there.
            // std::fs::rename overwrites files, but for directories it might fail if non-empty?
            // Ideally we remove shared_path first if it exists.
            if shared_path.is_dir() {
                std::fs::remove_dir_all(&shared_path)?;
            } else {
                std::fs::remove_file(&shared_path).ok();
            }
        }

        std::fs::rename(&absolute_source_path, &shared_path)?;
    }

    // 5. Link in all worktrees
    let count = worktrees.len();
    for worktree in worktrees {
        // Compute target path in this worktree
        // We know `relative_path` relative to WHERE?
        // The user gave `path_str` relative to CWD.
        // We need `path` relative to `repo_root` to apply to other worktrees.
        // But `repo_root` might be different for each worktree (if main repo vs worktree dir).
        // Wait, `get_repo_root` returns the top-level of the current worktree.
        // We need the path relative to the repository root.

        let target_in_worktree = worktree.join(path_relative_to_root);

        // Remove existing target
        if target_in_worktree.exists() || is_symlink(&target_in_worktree) {
            if target_in_worktree.is_dir() && !is_symlink(&target_in_worktree) {
                std::fs::remove_dir_all(&target_in_worktree)?;
            } else {
                std::fs::remove_file(&target_in_worktree)?;
            }
        }

        // Create symlink
        // target -> shared_path
        std::os::unix::fs::symlink(&shared_path, &target_in_worktree)?;
    }

    println!("Shared {path_str} across {count} worktrees");
    Ok(())
}
