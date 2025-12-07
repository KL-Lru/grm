use std::path::Path;
use std::sync::Arc;

use crate::configs::Config;
use crate::core::RepoInfo;
use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::errors::GrmError;

pub struct SplitWorktreeUseCase {
    git: Arc<dyn GitRepository>,
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl SplitWorktreeUseCase {
    pub fn new(
        git: Arc<dyn GitRepository>,
        fs: Arc<dyn FileSystem>,
        ui: Arc<dyn UserInteraction>,
    ) -> Self {
        Self { git, fs, ui }
    }

    pub fn execute(&self, config: &Config, branch: &str) -> Result<(), GrmError> {
        let repo_root = self
            .git
            .get_repository_root()
            .map_err(|_| GrmError::NotInManagedRepository)?;
        let remote_url = self
            .git
            .get_remote_url(&repo_root)
            .map_err(|_| GrmError::NotInManagedRepository)?;
        let repo_info = RepoInfo::from_url(&remote_url)?;

        let dest_path = repo_info.build_repo_path(config.root(), branch);

        if self.fs.exists(&dest_path) {
            return Err(GrmError::AlreadyExists(dest_path.display().to_string()));
        }

        if let Some(parent) = dest_path.parent() {
            self.fs.create_dir(parent)?;
        }

        let create_new = if self.git.local_branch_exists(branch)? {
            false
        } else {
            !(self.git.remote_branch_exists(&remote_url, branch)?)
        };

        self.git.add_worktree(&dest_path, branch, create_new)?;

        self.ui.print(&dest_path.display().to_string());

        let shared_root = config
            .root()
            .join(".shared")
            .join(&repo_info.host)
            .join(&repo_info.user)
            .join(&repo_info.repo);

        if self.fs.exists(&shared_root) {
            self.link_shared_files(&shared_root, &dest_path, &shared_root)?;
        }

        Ok(())
    }

    fn link_shared_files(
        &self,
        current_dir: &Path,
        worktree_root: &Path,
        shared_root: &Path,
    ) -> Result<(), GrmError> {
        if !current_dir.is_dir() {
            return Ok(());
        }

        for path in self.fs.read_dir(current_dir)? {
            if path.is_dir() {
                self.link_shared_files(&path, worktree_root, shared_root)?;
            } else {
                let relative_path = path
                    .strip_prefix(shared_root)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                let target_path = worktree_root.join(relative_path);

                if let Some(parent) = target_path.parent() {
                    self.fs.create_dir(parent)?;
                }

                if self.fs.exists(&target_path) {
                    if target_path.is_dir() {
                        self.fs.remove_dir(&target_path)?;
                    } else {
                        self.fs.remove_file(&target_path)?;
                    }
                }

                self.fs.create_symlink(&path, &target_path)?;
            }
        }
        Ok(())
    }
}
