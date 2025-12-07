use std::path::PathBuf;
use std::sync::Arc;

use crate::configs::Config;
use crate::core::ports::{FileSystem, UserInteraction};
use crate::core::{RepoInfo, RepoScanner};
use crate::errors::GrmError;

pub struct RemoveRepositoryUseCase {
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl RemoveRepositoryUseCase {
    pub fn new(fs: Arc<dyn FileSystem>, ui: Arc<dyn UserInteraction>) -> Self {
        Self { fs, ui }
    }

    pub fn execute(&self, config: &Config, url: &str, force: bool) -> Result<(), GrmError> {
        let root = config.root();
        let repo_info = RepoInfo::from_url(url)?;
        let scanner = RepoScanner::new(Arc::clone(&self.fs));

        let matching_repos = scanner.find_repositories(root, &repo_info)?;

        if matching_repos.is_empty() {
            let searched_path = root.join(&repo_info.host).join(&repo_info.user);
            return Err(GrmError::UnmanagedRepository {
                url: url.to_string(),
                searched_path: searched_path.display().to_string(),
            });
        }

        if !self.prompt_confirmation(&matching_repos, force)? {
            return Err(GrmError::UserCancelled);
        }

        self.remove_repositories(&matching_repos)?;

        self.ui.print(&format!(
            "\nSuccessfully removed {} repository(ies).",
            matching_repos.len()
        ));

        Ok(())
    }

    fn prompt_confirmation(&self, repositories: &[PathBuf], force: bool) -> Result<bool, GrmError> {
        if force {
            return Ok(true);
        }

        self.ui.print("The following repositories will be deleted:");
        for repo in repositories {
            self.ui.print(&format!("  - {}", repo.display()));
        }
        self.ui.print("");

        self.ui
            .confirm("Do you want to continue?")
            .map_err(GrmError::from)
    }

    fn remove_repositories(&self, repositories: &[PathBuf]) -> Result<(), GrmError> {
        for repo in repositories {
            if self.fs.is_symlink(repo) {
                self.ui.print_error(&format!(
                    "Warning: Skipping symlink: {} (unexpected, should have been filtered)",
                    repo.display()
                ));
                continue;
            }

            self.fs.remove_dir(repo)?;
            self.ui.print(&format!("Removed: {}", repo.display()));
        }
        Ok(())
    }
}
