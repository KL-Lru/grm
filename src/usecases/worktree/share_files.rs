use std::path::PathBuf;
use std::sync::Arc;

use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::configs::Config;
use crate::core::RepoInfo;
use crate::errors::GrmError;

pub struct ShareFilesUseCase {
    git: Arc<dyn GitRepository>,
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl ShareFilesUseCase {
    pub fn new(
        git: Arc<dyn GitRepository>,
        fs: Arc<dyn FileSystem>,
        ui: Arc<dyn UserInteraction>,
    ) -> Self {
        Self { git, fs, ui }
    }

    pub fn execute(&self, config: &Config, path_str: &str) -> Result<(), GrmError> {
        let relative_path = PathBuf::from(path_str);

        let repo_root = self
            .git
            .get_repository_root()
            .map_err(|_| GrmError::NotInManagedRepository)?;
        let remote_url = self
            .git
            .get_remote_url(&repo_root)
            .map_err(|_| GrmError::NotInManagedRepository)?;
        let repo_info = RepoInfo::from_url(&remote_url)?;

        let current_dir = self.fs.current_dir()?;
        let absolute_source_path = current_dir.join(&relative_path);

        if !self.fs.exists(&absolute_source_path) {
            return Err(GrmError::NotFound(format!(
                "File/Directory not found: {path_str}"
            )));
        }

        let shared_path = repo_info.build_shared_path(config.root(), &relative_path);

        let worktrees = self.git.list_worktrees()?;
        let path_relative_to_root = absolute_source_path
            .strip_prefix(&repo_root)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut conflicts = Vec::new();
        for worktree in &worktrees {
            let target_in_worktree = worktree.join(path_relative_to_root);

            if let Ok(canon_target) = self.fs.normalize(&target_in_worktree, &current_dir)
                && let Ok(canon_source) = self.fs.normalize(&absolute_source_path, &current_dir)
                && canon_target == canon_source
            {
                continue;
            }

            if self.fs.exists(&target_in_worktree) && !self.fs.is_symlink(&target_in_worktree) {
                conflicts.push(target_in_worktree);
            }
        }

        if !conflicts.is_empty() {
            self.ui.print("The following files will be overwritten:");
            for conflict in &conflicts {
                self.ui.print(&format!("  {}", conflict.display()));
            }

            if !self.ui.confirm("Do you want to continue?")? {
                return Err(GrmError::UserCancelled);
            }
        }

        if self.fs.is_symlink(&absolute_source_path)
            && let Ok(canon_source) = self.fs.normalize(&absolute_source_path, &current_dir)
            && let Ok(canon_shared) = self.fs.normalize(&shared_path, &current_dir)
            && canon_source == canon_shared
        {
            self.ui.print(&format!("{path_str} is already shared."));
            return Ok(());
        }

        if !self.fs.is_symlink(&absolute_source_path) {
            if let Some(parent) = shared_path.parent() {
                self.fs.create_dir(parent)?;
            }

            if self.fs.exists(&shared_path) {
                if shared_path.is_dir() {
                    self.fs.remove_dir(&shared_path)?;
                } else {
                    self.fs.remove_file(&shared_path).ok();
                }
            }

            self.fs.rename(&absolute_source_path, &shared_path)?;
        }

        let count = worktrees.len();
        for worktree in worktrees {
            let target_in_worktree = worktree.join(path_relative_to_root);

            if self.fs.exists(&target_in_worktree) || self.fs.is_symlink(&target_in_worktree) {
                if target_in_worktree.is_dir() && !self.fs.is_symlink(&target_in_worktree) {
                    self.fs.remove_dir(&target_in_worktree)?;
                } else {
                    self.fs.remove_file(&target_in_worktree)?;
                }
            }

            self.fs.create_symlink(&shared_path, &target_in_worktree)?;
        }

        self.ui
            .print(&format!("Shared {path_str} across {count} worktrees"));
        Ok(())
    }
}
