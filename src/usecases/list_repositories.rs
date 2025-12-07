use std::sync::Arc;

use crate::configs::Config;
use crate::core::RepoScanner;
use crate::core::ports::{FileSystem, UserInteraction};
use crate::errors::GrmError;

pub struct ListRepositoriesUseCase {
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl ListRepositoriesUseCase {
    pub fn new(fs: Arc<dyn FileSystem>, ui: Arc<dyn UserInteraction>) -> Self {
        Self { fs, ui }
    }

    pub fn execute(&self, config: &Config, full_path: bool) -> Result<(), GrmError> {
        let root = config.root();
        let scanner = RepoScanner::new(Arc::clone(&self.fs));

        if !self.fs.exists(root) {
            self.ui.print("Nothing to display");
            return Ok(());
        }

        let mut repositories = scanner.scan_repositories(root)?;

        if repositories.is_empty() {
            self.ui.print("Nothing to display");
            return Ok(());
        }

        repositories.sort();

        for repo in repositories {
            if full_path {
                self.ui.print(&repo.display().to_string());
            } else {
                match repo.strip_prefix(root) {
                    Ok(relative) => self.ui.print(&relative.display().to_string()),
                    Err(_) => self.ui.print(&repo.display().to_string()),
                }
            }
        }

        Ok(())
    }
}
