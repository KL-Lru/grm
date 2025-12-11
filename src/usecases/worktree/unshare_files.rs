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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockFileSystem, MockGitRepository, MockUserInteraction};

    #[test]
    fn test_unshare_success() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        let repo_root = PathBuf::from("/test_root/github.com/user/repo+main");
        mock_git.set_repo_root(&repo_root);

        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo(&repo_root);
        mock_fs.add_dir("/test_root/.shared");
        mock_fs.add_dir("/test_root/.shared/github.com");
        mock_fs.add_dir("/test_root/.shared/github.com/user");
        mock_fs.add_dir("/test_root/.shared/github.com/user/repo");

        // Set current directory to repo root
        mock_fs.set_current_dir(&repo_root);

        // Setup: Shared file with symlinks in multiple worktrees
        let shared_file = PathBuf::from("/test_root/.shared/github.com/user/repo/test.txt");
        mock_fs.add_file(&shared_file);
        mock_fs.add_symlink(&repo_root.join("test.txt"), &shared_file);

        let worktree = PathBuf::from("/test_root/github.com/user/repo+feature");
        mock_fs.add_git_repo(&worktree);
        mock_fs.add_symlink(&worktree.join("test.txt"), &shared_file);

        let usecase = UnshareFilesUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "test.txt");

        // Assert
        assert!(result.is_ok());
        let messages = mock_ui.get_printed_messages();
        assert!(messages
            .iter()
            .any(|m| m.contains("Unshared 2 file(s) from all worktrees")));
    }

    #[test]
    fn test_unshare_no_files() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        let repo_root = PathBuf::from("/test_root/github.com/user/repo+main");
        mock_git.set_repo_root(&repo_root);

        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo(&repo_root);
        mock_fs.add_dir("/test_root/.shared");
        mock_fs.add_dir("/test_root/.shared/github.com");
        mock_fs.add_dir("/test_root/.shared/github.com/user");
        mock_fs.add_dir("/test_root/.shared/github.com/user/repo");

        // Set current directory to repo root
        mock_fs.set_current_dir(&repo_root);

        let usecase = UnshareFilesUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "nonexistent.txt");

        // Assert
        assert!(result.is_ok());
        let messages = mock_ui.get_printed_messages();
        assert!(messages
            .iter()
            .any(|m| m.contains("No shared files found to unshare")));
    }

    #[test]
    fn test_unshare_not_in_repo() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        // Don't set repo_root - simulates not being in a repository

        let usecase = UnshareFilesUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "test.txt");

        // Assert
        assert!(matches!(result, Err(GrmError::NotInManagedRepository)));
    }
}

