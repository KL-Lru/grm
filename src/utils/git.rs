use std::path::Path;
use std::process::{Command, Stdio};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
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
