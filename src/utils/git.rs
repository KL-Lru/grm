use std::path::Path;
use std::process::{Command, Stdio};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("Invalid git URL: {0}")]
    InvalidUrl(String),

    #[error("Failed to execute git command: {0}")]
    Execution(String),

    #[error("Git command failed with status {status}: {stderr}")]
    Failed { status: i32, stderr: String },

    #[error("Failed to parse git output: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Get the default branch name from a remote repository
///
/// Uses `git ls-remote --symref` to query the remote HEAD without cloning.
pub fn get_default_branch(url: &str) -> Result<String, GitError> {
    let output = Command::new("git")
        .args(["ls-remote", "--symref", url, "HEAD"])
        .output()
        .map_err(|e| GitError::Execution(format!("Failed to run git ls-remote: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Failed {
            status: output.status.code().unwrap_or(-1),
            stderr: stderr.to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse output like: "ref: refs/heads/main	HEAD"
    for line in stdout.lines() {
        if line.starts_with("ref:") && line.contains("HEAD") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                // Extract branch name from "refs/heads/branch"
                if let Some(branch) = parts[1].strip_prefix("refs/heads/") {
                    return Ok(branch.to_string());
                }
            }
        }
    }

    Err(GitError::Parse(
        "Could not determine default branch from git ls-remote output".to_string(),
    ))
}

/// Clone a git repository to a destination path
///
/// Executes `git clone` with progress output passed through to the terminal.
/// If branch is specified, clones only that branch with `--branch`.
pub fn clone_repo(url: &str, dest: &Path, branch: Option<&str>) -> Result<(), GitError> {
    let mut cmd = Command::new("git");
    cmd.arg("clone");

    if let Some(b) = branch {
        cmd.args(["--branch", b]);
    }

    cmd.arg(url);
    cmd.arg(dest);

    // Inherit stdio to show git progress
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd
        .status()
        .map_err(|e| GitError::Execution(format!("Failed to run git clone: {}", e)))?;

    if !status.success() {
        return Err(GitError::Failed {
            status: status.code().unwrap_or(-1),
            stderr: "git clone failed (see output above)".to_string(),
        });
    }

    Ok(())
}

/// Parsed repository information from a git URL
#[derive(Debug)]
pub struct RepoInfo {
    pub host: String,
    pub user: String,
    pub repo: String,
}

/// Parse a git URL (HTTPS or SSH) into components
///
/// Supports:
/// - `https://github.com/user/repo.git`
/// - `https://github.com/user/repo`
/// - `git@github.com:user/repo.git`
/// - `ssh://git@github.com/user/repo.git`
///
/// # Arguments
/// * `url` - Git repository URL to parse
///
/// # Returns
/// * `Ok(RepoInfo)` - Parsed repository information
/// * `Err(GitError::InvalidUrl)` - If URL format is not supported
pub fn parse_git_url(url: &str) -> Result<RepoInfo, GitError> {
    let url = url.trim();

    // (prefix, separator)
    let formats = [("https://", "/"), ("ssh://git@", "/"), ("git@", ":")];

    for (prefix, separator) in formats {
        if let Some(url_without_scheme) = url.strip_prefix(prefix) {
            let parts: Vec<&str> = url_without_scheme.splitn(2, separator).collect();
            if parts.len() != 2 {
                return Err(GitError::InvalidUrl(format!(
                    "Expected format: {prefix}host{separator}user/repo, got: {url}",
                )));
            }

            let host = parts[0];
            let path = parts[1];

            let path_parts: Vec<&str> = path.split('/').collect();
            if path_parts.len() < 2 {
                return Err(GitError::InvalidUrl(format!(
                    "Expected format: {prefix}host{separator}user/repo, got: {url}",
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
    }

    Err(GitError::InvalidUrl(format!(
        "Unsupported URL format. Supported: https://, git@, ssh://. Got: {url}",
    )))
}

/// Check if a directory is a git repository
///
/// A directory is considered a git repository if it contains a `.git` directory or file.
/// The `.git` can be either a directory (normal repository) or a file (submodule/worktree).
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// * `true` if the path contains a `.git` directory or file
/// * `false` otherwise
pub fn is_git_repository(path: &Path) -> bool {
    let git_path = path.join(".git");
    git_path.exists() && (git_path.is_dir() || git_path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_url_https() {
        let info = parse_git_url("https://github.com/user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");

        let info = parse_git_url("https://github.com/user/repo").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_parse_git_url_ssh() {
        let info = parse_git_url("git@github.com:user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");

        let info = parse_git_url("git@gitlab.com:user/repo").unwrap();
        assert_eq!(info.host, "gitlab.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_parse_git_url_ssh_protocol() {
        let info = parse_git_url("ssh://git@github.com/user/repo.git").unwrap();
        assert_eq!(info.host, "github.com");
        assert_eq!(info.user, "user");
        assert_eq!(info.repo, "repo");
    }

    #[test]
    fn test_parse_git_url_invalid() {
        assert!(parse_git_url("invalid").is_err());
        assert!(parse_git_url("https://github.com/user").is_err()); // Missing repo
        assert!(parse_git_url("git@github.com/user/repo.git").is_err()); // Wrong separator for short ssh
    }
}
