//! Mock Git repository for testing
//!
//! Provides a mock implementation of Git operations for testing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::core::ports::{GitError, GitRepository};

/// Mock Git repository for testing
pub struct MockGitRepository {
    repo_root: Mutex<Option<PathBuf>>,
    default_branches: Mutex<HashMap<String, String>>,
    remote_urls: Mutex<HashMap<PathBuf, String>>,
    local_branches: Mutex<Vec<String>>,
    remote_branches: Mutex<HashMap<String, Vec<String>>>,
    cloned_repos: Mutex<Vec<(String, PathBuf)>>,
    worktrees: Mutex<Vec<PathBuf>>,
    initialized_repos: Mutex<Vec<(PathBuf, String)>>,
    current_branches: Mutex<HashMap<PathBuf, String>>,
    force_error: Mutex<Option<GitError>>,
}

impl MockGitRepository {
    pub fn new() -> Self {
        Self {
            repo_root: Mutex::new(None),
            default_branches: Mutex::new(HashMap::new()),
            remote_urls: Mutex::new(HashMap::new()),
            local_branches: Mutex::new(Vec::new()),
            remote_branches: Mutex::new(HashMap::new()),
            cloned_repos: Mutex::new(Vec::new()),
            worktrees: Mutex::new(Vec::new()),
            initialized_repos: Mutex::new(Vec::new()),
            current_branches: Mutex::new(HashMap::new()),
            force_error: Mutex::new(None),
        }
    }

    /// Set the repository root for testing
    pub fn set_repo_root(&self, path: impl AsRef<Path>) {
        *self.repo_root.lock().unwrap() = Some(path.as_ref().to_path_buf());
    }

    /// Set the default branch for a URL
    pub fn set_default_branch(&self, url: impl Into<String>, branch: impl Into<String>) {
        self.default_branches
            .lock().unwrap()
            .insert(url.into(), branch.into());
    }

    /// Set the remote URL for a repository
    pub fn set_remote_url(&self, repo_path: impl AsRef<Path>, url: impl Into<String>) {
        self.remote_urls
            .lock().unwrap()
            .insert(repo_path.as_ref().to_path_buf(), url.into());
    }

    /// Add a local branch
    pub fn add_local_branch(&self, branch: impl Into<String>) {
        self.local_branches.lock().unwrap().push(branch.into());
    }

    /// Add a remote branch
    pub fn add_remote_branch(&self, url: impl Into<String>, branch: impl Into<String>) {
        let url = url.into();
        let branch = branch.into();
        self.remote_branches
            .lock().unwrap()
            .entry(url)
            .or_default()
            .push(branch);
    }

    /// Set the current branch for a repository path
    pub fn set_current_branch(&self, repo_path: impl AsRef<Path>, branch: impl Into<String>) {
        self.current_branches
            .lock().unwrap()
            .insert(repo_path.as_ref().to_path_buf(), branch.into());
    }

    /// Inject an error to be returned on the next operation
    pub fn inject_error(&self, error: GitError) {
        *self.force_error.lock().unwrap() = Some(error);
    }

    /// Get the list of cloned repositories (for assertions)
    pub fn get_cloned_repos(&self) -> Vec<(String, PathBuf)> {
        self.cloned_repos.lock().unwrap().clone()
    }

    /// Get the list of worktrees (for assertions)
    pub fn get_worktrees(&self) -> Vec<PathBuf> {
        self.worktrees.lock().unwrap().clone()
    }

    /// Get the list of initialized repositories (for assertions)
    pub fn get_initialized_repos(&self) -> Vec<(PathBuf, String)> {
        self.initialized_repos.lock().unwrap().clone()
    }

    fn check_error(&self) -> Result<(), GitError> {
        if let Some(err) = self.force_error.lock().unwrap().take() {
            return Err(err);
        }
        Ok(())
    }
}

impl Default for MockGitRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl GitRepository for MockGitRepository {
    fn get_default_branch(&self, url: &str) -> Result<String, GitError> {
        self.check_error()?;

        self.default_branches
            .lock().unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| GitError::Parse(format!("No default branch configured for {url}")))
    }

    fn get_repository_root(&self) -> Result<PathBuf, GitError> {
        self.check_error()?;

        self.repo_root
            .lock().unwrap()
            .clone()
            .ok_or_else(|| GitError::Parse("No repository root configured".into()))
    }

    fn get_remote_url(&self, repo_path: &Path) -> Result<String, GitError> {
        self.check_error()?;

        self.remote_urls
            .lock().unwrap()
            .get(repo_path)
            .cloned()
            .ok_or_else(|| {
                GitError::Parse(format!(
                    "No remote URL configured for {}",
                    repo_path.display()
                ))
            })
    }

    fn local_branch_exists(&self, branch: &str) -> Result<bool, GitError> {
        self.check_error()?;

        Ok(self.local_branches.lock().unwrap().contains(&branch.to_string()))
    }

    fn remote_branch_exists(&self, remote_url: &str, branch: &str) -> Result<bool, GitError> {
        self.check_error()?;

        Ok(self
            .remote_branches
            .lock().unwrap()
            .get(remote_url)
            .is_some_and(|branches| branches.contains(&branch.to_string())))
    }

    fn get_current_branch(&self, repo_path: &Path) -> Result<String, GitError> {
        self.check_error()?;

        self.current_branches
            .lock().unwrap()
            .get(repo_path)
            .cloned()
            .ok_or_else(|| {
                GitError::Parse(format!("No current branch set for {}", repo_path.display()))
            })
    }

    fn clone_repository(
        &self,
        url: &str,
        destination: &Path,
        _branch: Option<&str>,
    ) -> Result<(), GitError> {
        self.check_error()?;

        self.cloned_repos
            .lock().unwrap()
            .push((url.to_string(), destination.to_path_buf()));

        Ok(())
    }

    fn add_worktree(
        &self,
        worktree_path: &Path,
        branch: &str,
        create_new: bool,
    ) -> Result<(), GitError> {
        self.check_error()?;

        self.worktrees
            .lock().unwrap()
            .push(worktree_path.to_path_buf());

        if create_new {
            self.add_local_branch(branch);
        }

        Ok(())
    }

    fn remove_worktree(&self, worktree_path: &Path) -> Result<(), GitError> {
        self.check_error()?;

        let mut worktrees = self.worktrees.lock().unwrap();
        worktrees.retain(|p| p != worktree_path);

        Ok(())
    }

    fn init_repository(&self, destination: &Path, branch: &str) -> Result<(), GitError> {
        self.check_error()?;

        self.initialized_repos
            .lock().unwrap()
            .push((destination.to_path_buf(), branch.to_string()));

        Ok(())
    }
}
