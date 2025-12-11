//! Mock Git repository for testing
//!
//! Provides a mock implementation of Git operations for testing.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::core::ports::{GitError, GitRepository};

/// Mock Git repository for testing
pub struct MockGitRepository {
    repo_root: RefCell<Option<PathBuf>>,
    default_branches: RefCell<HashMap<String, String>>,
    remote_urls: RefCell<HashMap<PathBuf, String>>,
    local_branches: RefCell<Vec<String>>,
    remote_branches: RefCell<HashMap<String, Vec<String>>>,
    cloned_repos: RefCell<Vec<(String, PathBuf)>>,
    worktrees: RefCell<Vec<PathBuf>>,
    force_error: RefCell<Option<GitError>>,
}

impl MockGitRepository {
    pub fn new() -> Self {
        Self {
            repo_root: RefCell::new(None),
            default_branches: RefCell::new(HashMap::new()),
            remote_urls: RefCell::new(HashMap::new()),
            local_branches: RefCell::new(Vec::new()),
            remote_branches: RefCell::new(HashMap::new()),
            cloned_repos: RefCell::new(Vec::new()),
            worktrees: RefCell::new(Vec::new()),
            force_error: RefCell::new(None),
        }
    }

    /// Set the repository root for testing
    pub fn set_repo_root(&self, path: impl AsRef<Path>) {
        *self.repo_root.borrow_mut() = Some(path.as_ref().to_path_buf());
    }

    /// Set the default branch for a URL
    pub fn set_default_branch(&self, url: impl Into<String>, branch: impl Into<String>) {
        self.default_branches
            .borrow_mut()
            .insert(url.into(), branch.into());
    }

    /// Set the remote URL for a repository
    pub fn set_remote_url(&self, repo_path: impl AsRef<Path>, url: impl Into<String>) {
        self.remote_urls
            .borrow_mut()
            .insert(repo_path.as_ref().to_path_buf(), url.into());
    }

    /// Add a local branch
    pub fn add_local_branch(&self, branch: impl Into<String>) {
        self.local_branches.borrow_mut().push(branch.into());
    }

    /// Add a remote branch
    pub fn add_remote_branch(&self, url: impl Into<String>, branch: impl Into<String>) {
        let url = url.into();
        let branch = branch.into();
        self.remote_branches
            .borrow_mut()
            .entry(url)
            .or_default()
            .push(branch);
    }

    /// Inject an error to be returned on the next operation
    pub fn inject_error(&self, error: GitError) {
        *self.force_error.borrow_mut() = Some(error);
    }

    /// Get the list of cloned repositories (for assertions)
    pub fn get_cloned_repos(&self) -> Vec<(String, PathBuf)> {
        self.cloned_repos.borrow().clone()
    }

    /// Get the list of worktrees (for assertions)
    pub fn get_worktrees(&self) -> Vec<PathBuf> {
        self.worktrees.borrow().clone()
    }

    fn check_error(&self) -> Result<(), GitError> {
        if let Some(err) = self.force_error.borrow_mut().take() {
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
            .borrow()
            .get(url)
            .cloned()
            .ok_or_else(|| GitError::Parse(format!("No default branch configured for {}", url)))
    }

    fn get_repository_root(&self) -> Result<PathBuf, GitError> {
        self.check_error()?;

        self.repo_root
            .borrow()
            .clone()
            .ok_or_else(|| GitError::Parse("No repository root configured".into()))
    }

    fn get_remote_url(&self, repo_path: &Path) -> Result<String, GitError> {
        self.check_error()?;

        self.remote_urls
            .borrow()
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

        Ok(self.local_branches.borrow().contains(&branch.to_string()))
    }

    fn remote_branch_exists(&self, remote_url: &str, branch: &str) -> Result<bool, GitError> {
        self.check_error()?;

        Ok(self
            .remote_branches
            .borrow()
            .get(remote_url)
            .map(|branches| branches.contains(&branch.to_string()))
            .unwrap_or(false))
    }

    fn clone_repository(
        &self,
        url: &str,
        destination: &Path,
        _branch: Option<&str>,
    ) -> Result<(), GitError> {
        self.check_error()?;

        self.cloned_repos
            .borrow_mut()
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
            .borrow_mut()
            .push(worktree_path.to_path_buf());

        if create_new {
            self.add_local_branch(branch);
        }

        Ok(())
    }

    fn remove_worktree(&self, worktree_path: &Path) -> Result<(), GitError> {
        self.check_error()?;

        let mut worktrees = self.worktrees.borrow_mut();
        worktrees.retain(|p| p != worktree_path);

        Ok(())
    }
}
