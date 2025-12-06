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
}

/// Helper for executing git commands
struct GitCommand<'a> {
    args: Vec<&'a str>,
}

impl<'a> GitCommand<'a> {
    fn new(args: &[&'a str]) -> Self {
        Self {
            args: args.to_vec(),
        }
    }

    /// Execute the command and capture stdout/stderr
    fn output(&self) -> Result<std::process::Output, GitError> {
        let output = Command::new("git")
            .args(&self.args)
            .output()
            .map_err(|error| {
                GitError::Execution(format!(
                    "Failed to execute git {}: {}",
                    self.args.join(" "),
                    error
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::Failed {
                status: output.status.code().unwrap_or(-1),
                stderr: stderr.to_string(),
            });
        }

        Ok(output)
    }

    /// Execute the command with inherited stdio
    fn execute(&self) -> Result<(), GitError> {
        let status = Command::new("git")
            .args(&self.args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|error| {
                GitError::Execution(format!(
                    "Failed to execute git {}: {}",
                    self.args.join(" "),
                    error
                ))
            })?;

        if !status.success() {
            return Err(GitError::Failed {
                status: status.code().unwrap_or(-1),
                stderr: format!("git {} failed", self.args.join(" ")),
            });
        }

        Ok(())
    }
}
/// Get the default branch name from a remote repository
///
/// Uses `git ls-remote --symref` to query the remote HEAD without cloning.
pub fn get_default_branch(url: &str) -> Result<String, GitError> {
    let output = GitCommand::new(&["ls-remote", "--symref", url, "HEAD"]).output()?;

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
    let dest_path = dest.to_string_lossy();
    let mut args = vec!["clone", url, dest_path.as_ref()];

    if let Some(b) = branch {
        args.extend_from_slice(&["--branch", b]);
    }

    GitCommand::new(&args).execute()?;

    Ok(())
}
