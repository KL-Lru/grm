use std::sync::Arc;

use crate::core::ports::{GitRepository, UserInteraction};
use crate::configs::Config;
use crate::core::RepoInfo;
use crate::errors::GrmError;

pub struct RemoveWorktreeUseCase {
    git: Arc<dyn GitRepository>,
    ui: Arc<dyn UserInteraction>,
}

impl RemoveWorktreeUseCase {
    pub fn new(git: Arc<dyn GitRepository>, ui: Arc<dyn UserInteraction>) -> Self {
        Self { git, ui }
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

        let worktree_path = repo_info.build_repo_path(config.root(), branch);

        if !worktree_path.exists() {
            return Err(GrmError::NotFound(format!(
                "Worktree does not exist: {}",
                worktree_path.display()
            )));
        }

        self.git
            .remove_worktree(&worktree_path)
            .map_err(GrmError::Git)?;

        self.ui
            .print(&format!("Removed worktree: {}", worktree_path.display()));

        Ok(())
    }
}
