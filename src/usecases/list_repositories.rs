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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockFileSystem, MockUserInteraction};
    use std::path::PathBuf;

    #[test]
    fn test_list_repositories_success() {
        // Arrange
        let mock_fs = MockFileSystem::new();
        mock_fs.add_dir("/test_root");
        mock_fs.add_git_repo("/test_root/repo1");
        mock_fs.add_git_repo("/test_root/repo2");
        mock_fs.add_git_repo("/test_root/repo3");

        let mock_ui = Arc::new(MockUserInteraction::new());

        let usecase = ListRepositoriesUseCase::new(
            Arc::new(mock_fs),
            mock_ui.clone(),
        );

        let config = Config { root: PathBuf::from("/test_root") };

        // Act
        let result = usecase.execute(&config, false);

        // Assert
        assert!(result.is_ok());
        let messages = mock_ui.get_printed_messages();
        assert_eq!(messages.len(), 3);
        assert!(messages.contains(&"repo1".to_string()));
        assert!(messages.contains(&"repo2".to_string()));
        assert!(messages.contains(&"repo3".to_string()));
    }

    #[test]
    fn test_list_repositories_full_path() {
        // Arrange
        let mock_fs = MockFileSystem::new();
        mock_fs.add_dir("/test_root");
        mock_fs.add_git_repo("/test_root/repo1");
        mock_fs.add_dir("/test_root/nested");
        mock_fs.add_git_repo("/test_root/nested/repo2");

        let mock_ui = Arc::new(MockUserInteraction::new());

        let usecase = ListRepositoriesUseCase::new(
            Arc::new(mock_fs),
            mock_ui.clone(),
        );

        let config = Config { root: PathBuf::from("/test_root") };

        // Act
        let result = usecase.execute(&config, true);

        // Assert
        assert!(result.is_ok());
        let messages = mock_ui.get_printed_messages();
        assert_eq!(messages.len(), 2);
        assert!(messages.contains(&"/test_root/repo1".to_string()));
        assert!(messages.contains(&"/test_root/nested/repo2".to_string()));
    }

    #[test]
    fn test_list_repositories_empty() {
        // Arrange
        let mock_fs = MockFileSystem::new();
        mock_fs.add_dir("/test_root");

        let mock_ui = Arc::new(MockUserInteraction::new());

        let usecase = ListRepositoriesUseCase::new(
            Arc::new(mock_fs),
            mock_ui.clone(),
        );

        let config = Config { root: PathBuf::from("/test_root") };

        // Act
        let result = usecase.execute(&config, false);

        // Assert
        assert!(result.is_ok());
        let messages = mock_ui.get_printed_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "Nothing to display");
    }

    #[test]
    fn test_list_repositories_root_not_exists() {
        // Arrange
        let mock_fs = MockFileSystem::new();

        let mock_ui = Arc::new(MockUserInteraction::new());

        let usecase = ListRepositoriesUseCase::new(
            Arc::new(mock_fs),
            mock_ui.clone(),
        );

        let config = Config { root: PathBuf::from("/nonexistent_root") };

        // Act
        let result = usecase.execute(&config, false);

        // Assert
        assert!(result.is_ok());
        let messages = mock_ui.get_printed_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "Nothing to display");
    }
}
