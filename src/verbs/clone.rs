use std::path::PathBuf;

use crate::configs::Config;
use crate::errors::GrmError;
use crate::utils::git;
use crate::utils::git_url::{RepoInfo, parse_git_url};

/// Build the destination path for a cloned repository
fn build_dest_path(root: &std::path::Path, info: &RepoInfo, branch: &str) -> PathBuf {
    root.join(&info.host)
        .join(&info.user)
        .join(format!("{}+{}", info.repo, branch))
}

/// Execute the clone command
pub fn execute(url: &str, branch: Option<&str>) -> Result<(), GrmError> {
    // Parse the URL
    let repo_info = parse_git_url(url)?;

    // Determine the branch
    let branch_name = if let Some(b) = branch {
        b.to_string()
    } else {
        // Query remote for default branch
        git::get_default_branch(url)?
    };

    // Load configuration to get root directory
    let config = Config::load()?;
    let dest_path = build_dest_path(config.root(), &repo_info, &branch_name);

    // Check if directory already exists
    if dest_path.exists() {
        return Err(GrmError::AlreadyManaged(dest_path.display().to_string()));
    }

    // Create parent directories
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Clone the repository
    git::clone_repo(url, &dest_path, Some(&branch_name))?;

    println!("Repository cloned to: {}", dest_path.display());

    Ok(())
}
