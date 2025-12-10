use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Failed to execute git command: {0}")]
    Execution(String),

    #[error("Git command failed with status {status}: {stderr}")]
    Failed { status: i32, stderr: String },

    #[error("Failed to parse git output: {0}")]
    Parse(String),
}

pub trait GitRepository {
    fn get_default_branch(&self, url: &str) -> Result<String, GitError>;

    fn get_repository_root(&self) -> Result<PathBuf, GitError>;

    fn get_remote_url(&self, repo_path: &Path) -> Result<String, GitError>;

    fn local_branch_exists(&self, branch: &str) -> Result<bool, GitError>;

    fn remote_branch_exists(&self, remote_url: &str, branch: &str) -> Result<bool, GitError>;

    fn clone_repository(
        &self,
        url: &str,
        destination: &Path,
        branch: Option<&str>,
    ) -> Result<(), GitError>;

    fn add_worktree(
        &self,
        worktree_path: &Path,
        branch: &str,
        create_new: bool,
    ) -> Result<(), GitError>;

    fn remove_worktree(&self, worktree_path: &Path) -> Result<(), GitError>;
}
