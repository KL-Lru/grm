use std::path::PathBuf;
use thiserror::Error;

use crate::configs::Config;
use crate::utils::git;

#[derive(Debug, Error)]
pub enum CloneError {
    #[error("Invalid git URL: {0}")]
    InvalidUrl(String),

    #[error("Directory already exists: {0}")]
    DirectoryExists(String),

    #[error("Git operation failed: {0}")]
    Git(#[from] git::GitError),

    #[error("Config error: {0}")]
    Config(#[from] crate::configs::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Parsed repository information from a git URL
#[derive(Debug)]
struct RepoInfo {
    host: String,
    user: String,
    repo: String,
}

/// Parse a git URL (HTTPS or SSH) into components
///
/// Supports:
/// - `https://github.com/user/repo.git`
/// - `https://github.com/user/repo`
/// - `git@github.com:user/repo.git`
/// - `ssh://git@github.com/user/repo.git`
fn parse_git_url(url: &str) -> Result<RepoInfo, CloneError> {
    let url = url.trim();

    // HTTPS format: https://host/user/repo(.git)?
    if let Some(url_without_scheme) = url.strip_prefix("https://") {
        let parts: Vec<&str> = url_without_scheme.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(CloneError::InvalidUrl(format!(
                "Expected format: https://host/user/repo, got: {url}",
            )));
        }

        let host = parts[0];
        let path = parts[1];

        let path_parts: Vec<&str> = path.split('/').collect();
        if path_parts.len() < 2 {
            return Err(CloneError::InvalidUrl(format!(
                "Expected format: https://host/user/repo, got: {url}",
            )));
        }

        let user = path_parts[0];
        let repo = path_parts[1].trim_end_matches(".git");

        return Ok(RepoInfo {
            host: host.to_string(),
            user: user.to_string(),
            repo: repo.to_string(),
        });
    }

    // SSH format: git@host:user/repo(.git)?
    if let Some(url_without_scheme) = url.strip_prefix("git@") {
        let parts: Vec<&str> = url_without_scheme.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(CloneError::InvalidUrl(format!(
                "Expected format: git@host:user/repo, got: {url}",
            )));
        }

        let host = parts[0];
        let path = parts[1];

        let path_parts: Vec<&str> = path.split('/').collect();
        if path_parts.len() < 2 {
            return Err(CloneError::InvalidUrl(format!(
                "Expected format: git@host:user/repo, got: {url}",
            )));
        }

        let user = path_parts[0];
        let repo = path_parts[1].trim_end_matches(".git");

        return Ok(RepoInfo {
            host: host.to_string(),
            user: user.to_string(),
            repo: repo.to_string(),
        });
    }

    // ssh:// format: ssh://git@host/user/repo(.git)?
    if let Some(url_without_scheme) = url.strip_prefix("ssh://git@") {
        let parts: Vec<&str> = url_without_scheme.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(CloneError::InvalidUrl(format!(
                "Expected format: ssh://git@host/user/repo, got: {url}",
            )));
        }

        let host = parts[0];
        let path = parts[1];

        let path_parts: Vec<&str> = path.split('/').collect();
        if path_parts.len() < 2 {
            return Err(CloneError::InvalidUrl(format!(
                "Expected format: ssh://git@host/user/repo, got: {url}",
            )));
        }

        let user = path_parts[0];
        let repo = path_parts[1].trim_end_matches(".git");

        return Ok(RepoInfo {
            host: host.to_string(),
            user: user.to_string(),
            repo: repo.to_string(),
        });
    }

    Err(CloneError::InvalidUrl(format!(
        "Unsupported URL format. Supported: https://, git@, ssh://. Got: {url}",
    )))
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
