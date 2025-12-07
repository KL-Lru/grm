use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::core::ports::{GitError, GitRepository};

pub struct GitCli;

impl GitCli {
    pub fn new() -> Self {
        Self
    }

    fn run_command(args: &[&str]) -> Result<String, GitError> {
        match Command::new("git").args(args).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                Ok(stdout.trim().to_string())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);

                Err(GitError::Failed {
                    status: output.status.code().unwrap_or(-1),
                    stderr: stderr.trim().to_string(),
                })
            }

            Err(error) => {
                let message = format!("Failed to execute git {}: {}", args.join(" "), error);

                Err(GitError::Execution(message))
            }
        }
    }

    fn run_command_inherit(args: &[&str]) -> Result<(), GitError> {
        match Command::new("git")
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
        {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(GitError::Failed {
                status: status.code().unwrap_or(-1),
                stderr: format!("git {} failed", args.join(" ")),
            }),
            Err(error) => {
                let message = format!("Failed to execute git {}: {}", args.join(" "), error);

                Err(GitError::Execution(message))
            }
        }
    }
}

impl Default for GitCli {
    fn default() -> Self {
        Self::new()
    }
}

impl GitRepository for GitCli {
    fn get_default_branch(&self, url: &str) -> Result<String, GitError> {
        let output = Self::run_command(&["ls-remote", "--symref", url, "HEAD"])?;

        for line in output.lines() {
            // expected: ref: refs/heads/main HEAD
            if line.starts_with("ref:") && line.contains("HEAD") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2
                    && let Some(branch) = parts[1].strip_prefix("refs/heads/")
                {
                    return Ok(branch.to_string());
                }
            }
        }

        Err(GitError::Parse(
            "Could not determine default branch from git ls-remote output".to_string(),
        ))
    }

    fn get_repository_root(&self) -> Result<PathBuf, GitError> {
        let output = Self::run_command(&["rev-parse", "--show-toplevel"])?;

        if output.is_empty() {
            return Err(GitError::Parse(
                "Could not determine repository root".to_string(),
            ));
        }

        Ok(PathBuf::from(output))
    }

    fn get_remote_url(&self, repo_path: &Path) -> Result<String, GitError> {
        let output = Self::run_command(&[
            "-C",
            &repo_path.to_string_lossy(),
            "config",
            "--get",
            "remote.origin.url",
        ])?;

        if output.is_empty() {
            return Err(GitError::Parse("No remote URL found".to_string()));
        }

        Ok(output)
    }

    fn local_branch_exists(&self, branch: &str) -> Result<bool, GitError> {
        let ref_name = format!("refs/heads/{branch}");
        let result = Self::run_command(&["rev-parse", "--verify", &ref_name]);

        match result {
            Ok(_) => Ok(true),
            Err(GitError::Failed { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn remote_branch_exists(&self, remote_url: &str, branch: &str) -> Result<bool, GitError> {
        let ref_name = format!("refs/heads/{branch}");
        let output = Self::run_command(&["ls-remote", "--heads", remote_url, &ref_name])?;

        for line in output.lines() {
            if line.contains(&ref_name) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn clone_repository(
        &self,
        url: &str,
        destination: &Path,
        branch: Option<&str>,
    ) -> Result<(), GitError> {
        let dest_path = destination.to_string_lossy();
        let mut args = vec!["clone", url, dest_path.as_ref()];

        if let Some(b) = branch {
            args.extend_from_slice(&["--branch", b]);
        }

        Self::run_command_inherit(&args)
    }

    fn add_worktree(
        &self,
        worktree_path: &Path,
        branch: &str,
        create_new: bool,
    ) -> Result<(), GitError> {
        let worktree_path_str = worktree_path.to_string_lossy();
        let mut args = vec!["worktree", "add"];

        if create_new {
            args.extend_from_slice(&["-b", branch, worktree_path_str.as_ref()]);
        } else {
            args.extend_from_slice(&[worktree_path_str.as_ref(), branch]);
        }

        Self::run_command_inherit(&args)
    }

    fn remove_worktree(&self, worktree_path: &Path) -> Result<(), GitError> {
        let worktree_path_str = worktree_path.to_string_lossy();
        Self::run_command_inherit(&["worktree", "remove", worktree_path_str.as_ref()])
    }

    fn list_worktrees(&self) -> Result<Vec<PathBuf>, GitError> {
        let output = Self::run_command(&["worktree", "list", "--porcelain"])?;

        let mut worktrees = Vec::new();
        for line in output.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                worktrees.push(PathBuf::from(path));
            }
        }

        Ok(worktrees)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_dummy_repo(dir: &Path) {
        Command::new("git")
            .args(["init", "--initial-branch=test"])
            .current_dir(dir)
            .output()
            .expect("Failed to init git repo");

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

        std::fs::write(dir.join("README.md"), "# Dummy Repo").expect("Failed to write README.md");

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

        let url = format!("file://{}", temp_dir.path().display());
        let adapter = GitCli::new();
        let branch = adapter
            .get_default_branch(&url)
            .expect("Failed to get default branch");

        assert_eq!(branch, "test");
    }

    #[test]
    fn test_clone_repository_local() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("repo");
        std::fs::create_dir(&repo_dir).unwrap();
        setup_dummy_repo(&repo_dir);

        let clone_dest = temp_dir.path().join("clone");
        let url = format!("file://{}", repo_dir.display());

        let adapter = GitCli::new();
        adapter
            .clone_repository(&url, &clone_dest, None)
            .expect("Failed to clone repo");

        assert!(clone_dest.join(".git").exists());
        assert!(clone_dest.join("README.md").exists());
    }
}
