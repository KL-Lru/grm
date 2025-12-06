use std::path::{Path, PathBuf};
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

/// Get the git repository root from current directory
///
/// Uses `git rev-parse --show-toplevel` to find the repository root.
pub fn get_repo_root() -> Result<PathBuf, GitError> {
    let output = GitCommand::new(&["rev-parse", "--show-toplevel"]).output()?;
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if root.is_empty() {
        return Err(GitError::Parse(
            "Could not determine repository root".to_string(),
        ));
    }

    Ok(PathBuf::from(root))
}

/// Get remote URL for a repository
///
/// Uses `git config --get remote.origin.url` to get the remote URL.
pub fn get_remote_url(repo_path: &Path) -> Result<String, GitError> {
    let output = GitCommand::new(&[
        "-C",
        &repo_path.to_string_lossy(),
        "config",
        "--get",
        "remote.origin.url",
    ])
    .output()?;

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if url.is_empty() {
        return Err(GitError::Parse("No remote URL found".to_string()));
    }

    Ok(url)
}

/// Check if a branch exists locally
///
/// Uses `git rev-parse --verify` to check if a branch exists locally.
pub fn local_branch_exists(branch: &str) -> Result<bool, GitError> {
    let ref_name = format!("refs/heads/{branch}");
    let result = GitCommand::new(&["rev-parse", "--verify", &ref_name]).output();

    match result {
        Ok(_) => Ok(true),
        Err(GitError::Failed { .. }) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Check if a branch exists on remote
///
/// Uses `git ls-remote --heads` to check if a branch exists (exact match).
pub fn remote_branch_exists(remote_url: &str, branch: &str) -> Result<bool, GitError> {
    let ref_name = format!("refs/heads/{branch}");
    let output = GitCommand::new(&["ls-remote", "--heads", remote_url, &ref_name]).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for exact match
    for line in stdout.lines() {
        if line.contains(&ref_name) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Add a new worktree
///
/// Executes `git worktree add` to create a new worktree.
/// If `new_branch` is true, creates a new branch with `-b`.
pub fn add_worktree(path: &Path, branch: &str, new_branch: bool) -> Result<(), GitError> {
    let path_str = path.to_string_lossy();
    let mut args = vec!["worktree", "add"];

    if new_branch {
        args.extend_from_slice(&["-b", branch, path_str.as_ref()]);
    } else {
        args.extend_from_slice(&[path_str.as_ref(), branch]);
    }

    GitCommand::new(&args).execute()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_dummy_repo(dir: &Path) {
        // Initialize git repo
        Command::new("git")
            .args(["init", "--initial-branch=test"])
            .current_dir(dir)
            .output()
            .expect("Failed to init git repo");

        // Set local user config to avoid "Please tell me who you are." error
        Command::new("git")
            .args(["config", "user.email", "you@example.com"])
            .current_dir(dir)
            .output()
            .expect("Failed to set user.email");

        Command::new("git")
            .args(["config", "user.name", "Your Name"])
            .current_dir(dir)
            .output()
            .expect("Failed to set user.name");

        // Create a dummy file
        std::fs::write(dir.join("README.md"), "# Dummy Repo").expect("Failed to write README.md");

        // Commit it
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(dir)
            .output()
            .expect("Failed to git commit");
    }

    #[test]
    fn test_get_default_branch_local() {
        let temp_dir = TempDir::new().unwrap();
        setup_dummy_repo(temp_dir.path());

        // Use file:// URL for local repo
        let url = format!("file://{}", temp_dir.path().display());
        let branch = get_default_branch(&url).expect("Failed to get default branch");

        assert_eq!(branch, "test");
    }

    #[test]
    fn test_clone_repo_local() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("repo");
        std::fs::create_dir(&repo_dir).unwrap();
        setup_dummy_repo(&repo_dir);

        let clone_dest = temp_dir.path().join("clone");
        let url = format!("file://{}", repo_dir.display());

        clone_repo(&url, &clone_dest, None).expect("Failed to clone repo");

        assert!(clone_dest.join(".git").exists());
        assert!(clone_dest.join("README.md").exists());
    }
}
