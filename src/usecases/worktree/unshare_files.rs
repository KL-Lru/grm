use std::path::PathBuf;
use std::sync::Arc;

use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::configs::Config;
use crate::core::RepoInfo;
use crate::errors::GrmError;

pub struct UnshareFilesUseCase {
    git: Arc<dyn GitRepository>,
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl UnshareFilesUseCase {
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

        let shared_path = repo_info.build_shared_path(config.root(), &relative_path);

        let worktrees = self.git.list_worktrees()?;

        let current_dir = self.fs.current_dir()?;

        let canonical_shared_path = if self.fs.exists(&shared_path) {
            self.fs.normalize(&shared_path, &current_dir)?
        } else {
            shared_path.clone()
        };
        let absolute_target_path = current_dir.join(&relative_path);

        let path_relative_to_root = absolute_target_path
            .strip_prefix(&repo_root)
            .map_err(|_| GrmError::NotInManagedRepository)?
            .to_path_buf();

        let mut removed_count = 0;

        for worktree in worktrees {
            let target_in_worktree = worktree.join(&path_relative_to_root);

            if !self.fs.exists(&target_in_worktree) && !self.fs.is_symlink(&target_in_worktree) {
                continue;
            }

            if self.fs.is_symlink(&target_in_worktree) {
                let is_match = if let Ok(link_target) = std::fs::read_link(&target_in_worktree) {
                    link_target == shared_path
                        || (link_target.is_relative()
                            && target_in_worktree
                                .parent()
                                .map(|p| p.join(&link_target))
                                .unwrap_or(link_target)
                                == shared_path)
                        || (self.fs.exists(&shared_path)
                            && self.fs.normalize(&target_in_worktree, &current_dir).ok()
                                == self.fs.normalize(&canonical_shared_path, &current_dir).ok())
                } else {
                    false
                };

                if is_match {
                    self.fs.remove_file(&target_in_worktree)?;
                    removed_count += 1;
                }
            }
        }

        if removed_count > 0 {
            self.ui.print(&format!(
                "Unshared {path_str} from {removed_count} worktrees"
            ));
        }

        Ok(())
    }
}
