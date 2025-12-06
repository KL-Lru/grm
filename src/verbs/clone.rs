use std::path::PathBuf;
use thiserror::Error;

use crate::configs::Config;
use crate::utils::git_repository::{RepoInfo, parse_git_url};
use crate::utils::git;

#[derive(Debug, Error)]
pub enum CloneError {
    #[error("Directory already exists: {0}")]
    DirectoryExists(String),

    #[error("Git operation failed: {0}")]
    Git(#[from] git::GitError),

    #[error("Config error: {0}")]
    Config(#[from] crate::configs::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Build the destination path for a cloned repository
fn build_dest_path(root: &std::path::Path, info: &RepoInfo, branch: &str) -> PathBuf {
    root.join(&info.host)
        .join(&info.user)
        .join(format!("{}+{}", info.repo, branch))
}

/// Execute the clone command
pub fn execute(url: &str, branch: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
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
        return Err(Box::new(CloneError::DirectoryExists(
            dest_path.display().to_string(),
        )));
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
