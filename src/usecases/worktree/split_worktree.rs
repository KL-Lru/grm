use std::sync::Arc;

use crate::configs::Config;
use crate::core::RepoInfo;
use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::core::shared_resource::SharedResource;
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

        let already_exists = self.git.local_branch_exists(branch)?
            || self.git.remote_branch_exists(&remote_url, branch)?;

        self.git.add_worktree(&dest_path, branch, !already_exists)?;

        self.ui.print(&dest_path.display().to_string());

        let shared_resource =
            SharedResource::new(repo_info, Arc::clone(&self.fs), config.root().to_path_buf());
        shared_resource.mount(&repo_root)?;

        Ok(())
    }
}
