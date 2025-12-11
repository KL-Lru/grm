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

        let matching_repos = scanner.scan_worktrees(root, &repo_info)?;

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

            self.fs.remove(repo)?;
            self.ui.print(&format!("Removed: {}", repo.display()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockFileSystem, MockUserInteraction};

    #[test]
    fn test_remove_repository_with_confirmation() {
        // Arrange
        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo("/test_root/github.com/user/repo+main");

        let mock_ui = Arc::new(MockUserInteraction::new());
        mock_ui.set_confirm(true);

        let usecase = RemoveRepositoryUseCase::new(mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "https://github.com/user/repo", false);

        // Assert
        assert!(result.is_ok());
        assert!(!mock_fs.exists(PathBuf::from("/test_root/github.com/user/repo+main").as_ref()));
        let messages = mock_ui.get_printed_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("Successfully removed 1 repository"))
        );
    }

    #[test]
    fn test_remove_repository_force() {
        // Arrange
        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo("/test_root/github.com/user/repo+main");

        let mock_ui = Arc::new(MockUserInteraction::new());

        let usecase = RemoveRepositoryUseCase::new(mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "https://github.com/user/repo", true);

        // Assert
        assert!(result.is_ok());
        assert!(!mock_fs.exists(PathBuf::from("/test_root/github.com/user/repo+main").as_ref()));
        let messages = mock_ui.get_printed_messages();
        assert!(
            !messages
                .iter()
                .any(|m| m.contains("The following repositories will be deleted"))
        );
    }

    #[test]
    fn test_remove_repository_user_cancelled() {
        // Arrange
        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo("/test_root/github.com/user/repo+main");

        let mock_ui = Arc::new(MockUserInteraction::new());
        mock_ui.set_confirm(false);

        let usecase = RemoveRepositoryUseCase::new(mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "https://github.com/user/repo", false);

        // Assert
        assert!(matches!(result, Err(GrmError::UserCancelled)));
        assert!(mock_fs.exists(PathBuf::from("/test_root/github.com/user/repo+main").as_ref()));
    }

    #[test]
    fn test_remove_repository_not_found() {
        // Arrange
        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");

        let mock_ui = Arc::new(MockUserInteraction::new());

        let usecase = RemoveRepositoryUseCase::new(mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "https://github.com/user/nonexistent", false);

        // Assert
        assert!(matches!(result, Err(GrmError::UnmanagedRepository { .. })));
    }

    #[test]
    fn test_remove_multiple_worktrees() {
        // Arrange
        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo("/test_root/github.com/user/repo+main");
        mock_fs.add_git_repo("/test_root/github.com/user/repo+feature");
        mock_fs.add_git_repo("/test_root/github.com/user/repo+dev");

        let mock_ui = Arc::new(MockUserInteraction::new());
        mock_ui.set_confirm(true);

        let usecase = RemoveRepositoryUseCase::new(mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "https://github.com/user/repo", false);

        // Assert
        assert!(result.is_ok());
        assert!(!mock_fs.exists(PathBuf::from("/test_root/github.com/user/repo+main").as_ref()));
        assert!(!mock_fs.exists(PathBuf::from("/test_root/github.com/user/repo+feature").as_ref()));
        assert!(!mock_fs.exists(PathBuf::from("/test_root/github.com/user/repo+dev").as_ref()));
        let messages = mock_ui.get_printed_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("Successfully removed 3 repository"))
        );
    }
}
