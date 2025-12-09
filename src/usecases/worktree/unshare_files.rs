use std::path::PathBuf;
use std::sync::Arc;

use crate::configs::Config;
use crate::core::RepoInfo;
use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::core::shared_resource::SharedResource;
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
        let repo_info = RepoInfo::from_path(config.root(), &repo_root)?;
        let resource = SharedResource::new(
            repo_info.clone(),
            Arc::clone(&self.fs),
            config.root().to_path_buf(),
        );

        match resource.unshare(&repo_root, &relative_path) {
            Ok(removed_count) => {
                if removed_count == 0 {
                    self.ui.print("No shared files found to unshare.");
                } else {
                    self.ui.print(&format!(
                        "Unshared {removed_count} file(s) from all worktrees.",
                    ));
                }

                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}
