use std::path::PathBuf;
use std::sync::Arc;

use crate::configs::Config;
use crate::core::RepoInfo;
use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::core::shared_resource::SharedResource;
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
        let repo_root = self
            .git
            .get_repository_root()
            .map_err(|_| GrmError::NotInManagedRepository)?;
        let repo_info = RepoInfo::from_path(config.root(), &repo_root)?;

        let relative_path = PathBuf::from(path_str);
        let current_dir = self.fs.current_dir()?;
        let absolute_source_path = current_dir.join(&relative_path);

        if !self.fs.exists(&absolute_source_path) {
            return Err(GrmError::NotFound(format!(
                "File/Directory not found: {path_str}"
            )));
        }

        let resource =
            SharedResource::new(repo_info, Arc::clone(&self.fs), config.root().to_path_buf());

        let conflicts = resource.conflicts(&repo_root, &relative_path)?;
        if !conflicts.is_empty() {
            self.ui.print("The following files will be overwritten:");
            for conflict in &conflicts {
                self.ui.print(&format!("  {}", conflict.display()));
            }

            if !self.ui.confirm("Do you want to continue?")? {
                return Err(GrmError::UserCancelled);
            }
        }

        resource.share(&repo_root, &relative_path)?;

        self.ui
            .print(&format!("Shared {path_str} across worktrees"));
        Ok(())
    }
}
