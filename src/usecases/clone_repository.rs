use std::path::PathBuf;
use std::sync::Arc;

use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::configs::Config;
use crate::core::RepoInfo;
use crate::errors::GrmError;

pub struct CloneRepositoryUseCase {
    git: Arc<dyn GitRepository>,
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl CloneRepositoryUseCase {
    pub fn new(
        git: Arc<dyn GitRepository>,
        fs: Arc<dyn FileSystem>,
        ui: Arc<dyn UserInteraction>,
    ) -> Self {
        Self { git, fs, ui }
    }

    pub fn execute(
        &self,
        config: &Config,
        url: &str,
        branch: Option<&str>,
    ) -> Result<PathBuf, GrmError> {
        let repo_info = RepoInfo::from_url(url)?;

        let branch_name = if let Some(b) = branch {
            b.to_string()
        } else {
            self.git.get_default_branch(url)?
        };

        let dest_path = repo_info.build_repo_path(config.root(), &branch_name);

        if self.fs.exists(&dest_path) {
            return Err(GrmError::AlreadyExists(dest_path.display().to_string()));
        }

        if let Some(parent) = dest_path.parent() {
            self.fs.create_dir(parent)?;
        }

        self.git
            .clone_repository(url, &dest_path, Some(&branch_name))?;

        self.ui
            .print(&format!("Repository cloned to: {}", dest_path.display()));

        Ok(dest_path)
    }
}
