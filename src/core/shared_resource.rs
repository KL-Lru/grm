use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    core::{RepoInfo, RepoScanner, ports::FileSystem},
    errors::GrmError,
};

pub struct SharedResource {
    repo_info: RepoInfo,
    fs: Arc<dyn FileSystem>,
    scanner: RepoScanner,
    root: PathBuf,
}

impl SharedResource {
    pub fn new(repo_info: RepoInfo, fs: Arc<dyn FileSystem>, root: PathBuf) -> Self {
        let scanner = RepoScanner::new(Arc::clone(&fs));
        Self {
            repo_info,
            fs,
            scanner,
            root,
        }
    }

    /// Check if a file or directory has conflicts in other worktrees
    ///
    /// # Arguments
    /// * `repo_root` - The root directory of the repository
    /// * `relative_path` - Path relative to the repository root
    ///
    /// # Returns
    /// * `Ok(Vec<PathBuf>)` - List of conflicting paths in other worktrees
    /// * `Err(GrmError)` - If an error occurs
    pub fn conflicts(
        &self,
        repo_root: &Path,
        relative_path: &Path,
    ) -> Result<Vec<PathBuf>, GrmError> {
        let current_dir = self.fs.current_dir()?;
        let file = self.fs.normalize(relative_path, &current_dir)?;
        let repo_relative_path = file
            .strip_prefix(repo_root)
            .map_err(|e| GrmError::NotFound(format!("{e}")))?;

        let shared_path = self
            .repo_info
            .build_shared_path(&self.root, repo_relative_path);
        if !self.fs.exists(&shared_path) {
            return Ok(Vec::new());
        }

        let mut conflicts = Vec::new();
        let worktrees = self.scanner.scan_worktrees(&self.root, &self.repo_info)?;
        for worktree in &worktrees {
            let target_in_worktree = worktree.join(repo_relative_path);
            if file == target_in_worktree {
                continue;
            }
            if self.fs.exists(&target_in_worktree) || self.fs.is_symlink(&target_in_worktree) {
                conflicts.push(target_in_worktree);
            }
        }

        Ok(conflicts)
    }

    /// Mount a shared file or directory for new worktrees
    ///
    /// # Arguments
    /// * `repo_root` - The root directory for managed repositories
    pub fn mount(&self, repo_root: &Path) -> Result<(), GrmError> {
        let shared_root = self.repo_info.build_shared_path(&self.root, Path::new(""));

        if !self.fs.exists(&shared_root) {
            return Err(GrmError::NotFound(format!(
                "Shared storage not found at {}",
                shared_root.display()
            )));
        }

        let mut queue = vec![shared_root.clone()];
        while let Some(current_dir) = queue.pop() {
            for entry in self.fs.read_dir(&current_dir)? {
                if self.fs.is_dir(&entry) {
                    self.fs.create_dir(&repo_root.join(&entry))?;
                    queue.push(entry);
                } else {
                    let relative_path = entry
                        .strip_prefix(&shared_root)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                    let target_path = repo_root.join(relative_path);

                    if self.fs.exists(&target_path) || self.fs.is_symlink(&target_path) {
                        self.fs.remove(&target_path)?;
                    }

                    self.fs.create_symlink(&entry, &target_path)?;
                }
            }
        }
        Ok(())
    }

    /// Share a file or directory across all worktrees
    ///
    /// # Arguments
    /// * `repo_root` - The root directory for managed repositories
    /// * `repo_relative_path` - Path relative to the repository root
    ///
    /// # Returns
    /// * `Ok(())` - Successfully shared the resource
    /// * `Err(GrmError)` - If sharing fails
    pub fn share(&self, repo_root: &Path, relative_path: &Path) -> Result<(), GrmError> {
        let current_dir = self.fs.current_dir()?;
        let file = self.fs.normalize(relative_path, &current_dir)?;
        let repo_relative_path = file
            .strip_prefix(repo_root)
            .map_err(|e| GrmError::NotFound(format!("{e}")))?;
        let shared_path = self
            .repo_info
            .build_shared_path(&self.root, repo_relative_path);

        if !self.fs.exists(&file) {
            return Err(GrmError::NotFound(format!(
                "File/Directory not found: {}",
                relative_path.display()
            )));
        }

        // Check if already shared
        if self.fs.is_symlink(&file) {
            return Ok(());
        }

        // Move the file to shared storage if it's not a symlink
        if let Some(parent) = shared_path.parent() {
            self.fs.create_dir(parent)?;
        }

        if self.fs.exists(&shared_path) {
            self.fs.remove(&shared_path)?;
        }

        self.fs.rename(&file, &shared_path)?;
        let worktrees = self.scanner.scan_worktrees(&self.root, &self.repo_info)?;

        // Create symlinks in all worktrees
        for worktree in &worktrees {
            let target_in_worktree = worktree.join(relative_path);

            if self.fs.exists(&target_in_worktree) || self.fs.is_symlink(&target_in_worktree) {
                self.fs.remove(&target_in_worktree)?;
            }

            self.fs.create_symlink(&shared_path, &target_in_worktree)?;
        }

        Ok(())
    }

    /// Unshare a file or directory from worktrees
    ///
    /// # Arguments
    /// * `repo_root` - The root directory for managed repositories
    /// * `repo_relative_path` - Path relative to the repository root
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of symlinks removed
    /// * `Err(GrmError)` - If unsharing fails
    pub fn unshare(&self, repo_root: &Path, relative_path: &Path) -> Result<usize, GrmError> {
        let current_dir = self.fs.current_dir()?;
        let file = self.fs.normalize(relative_path, &current_dir)?;

        let repo_relative_path = file
            .strip_prefix(repo_root)
            .map_err(|e| GrmError::NotFound(format!("{e}")))?;

        let mut removed_count = 0;

        let worktrees = self.scanner.scan_worktrees(&self.root, &self.repo_info)?;
        for worktree in &worktrees {
            let target_in_worktree = worktree.join(repo_relative_path);

            if !self.fs.exists(&target_in_worktree) && !self.fs.is_symlink(&target_in_worktree) {
                continue;
            }

            if self.fs.is_symlink(&target_in_worktree) {
                self.fs.remove(&target_in_worktree)?;
                removed_count += 1;
            }
        }

        Ok(removed_count)
    }

    /// Isolate a shared file/directory in a specific worktree
    ///
    /// # Arguments
    /// * `repo_root` - The root directory for managed repositories
    /// * `repo_relative_path` - Path relative to the repository root
    /// * `worktree_path` - The worktree path where to isolate the file
    ///
    /// # Returns
    /// * `Ok(())` - Successfully isolated the resource
    /// * `Err(GrmError)` - If isolation fails
    pub fn isolate(&self, repo_root: &Path, relative_path: &Path) -> Result<(), GrmError> {
        let current_dir = self.fs.current_dir()?;
        let file = self.fs.normalize(relative_path, &current_dir)?;
        let repo_relative_path = file
            .strip_prefix(repo_root)
            .map_err(|e| GrmError::NotFound(format!("{e}")))?;

        let shared_path = self
            .repo_info
            .build_shared_path(&self.root, repo_relative_path);
        let absolute_target_path = repo_root.join(repo_relative_path);

        if !self.fs.exists(&absolute_target_path) {
            return Err(GrmError::NotFound(format!(
                "File/Directory not found: {}",
                repo_relative_path.display()
            )));
        }

        if !self.fs.is_symlink(&absolute_target_path) {
            return Ok(());
        }

        if !self.fs.exists(&shared_path) {
            return Err(GrmError::NotFound(format!(
                "Shared storage not found at {}",
                shared_path.display()
            )));
        }

        self.fs.remove(&absolute_target_path)?;
        self.fs.copy(&shared_path, &absolute_target_path)?;

        Ok(())
    }
}
