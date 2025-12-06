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

    // Check availability of shared files and link them
    let shared_root = config
        .root()
        .join(".shared")
        .join(&repo_info.host)
        .join(&repo_info.user)
        .join(&repo_info.repo);

    if shared_root.exists() {
        link_shared_files(&shared_root, &dest_path, &shared_root)?;
    }

    Ok(())
}

fn link_shared_files(
    current_dir: &std::path::Path,
    worktree_root: &std::path::Path,
    shared_root: &std::path::Path,
) -> Result<(), GrmError> {
    if !current_dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            link_shared_files(&path, worktree_root, shared_root)?;
        } else {
            // It's a file, create symlink in worktree
            let relative_path = path
                .strip_prefix(shared_root)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            let target_path = worktree_root.join(relative_path);

            // Ensure parent dir exists
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Remove if exists
            if target_path.exists() {
                if target_path.is_dir() {
                    std::fs::remove_dir_all(&target_path)?;
                } else {
                    std::fs::remove_file(&target_path)?;
                }
            }

            std::os::unix::fs::symlink(&path, &target_path)?;
        }
    }
    Ok(())
}
