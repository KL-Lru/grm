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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockFileSystem, MockGitRepository, MockUserInteraction};

    #[test]
    fn test_share_file_success() {
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
        mock_fs.add_file(&repo_root.join("test.txt"));

        let usecase = ShareFilesUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "test.txt");

        // Assert
        if let Err(ref e) = result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok(), "Failed with error: {:?}", result.err());
        let messages = mock_ui.get_printed_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("Shared test.txt across worktrees"))
        );
    }

    #[test]
    fn test_share_file_with_conflicts_confirmed() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());
        mock_ui.set_confirm(true);

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

        // Setup: File is already shared (exists in shared storage and is a symlink in repo+main)
        let shared_file = PathBuf::from("/test_root/.shared/github.com/user/repo/test.txt");
        mock_fs.add_file(&shared_file);
        mock_fs.add_symlink(&repo_root.join("test.txt"), &shared_file);

        // Add conflicting worktree with a real file (not a symlink)
        let worktree = PathBuf::from("/test_root/github.com/user/repo+feature");
        mock_fs.add_git_repo(&worktree);
        mock_fs.add_file(&worktree.join("test.txt"));

        let usecase = ShareFilesUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "test.txt");

        // Assert
        assert!(result.is_ok());
        let messages = mock_ui.get_printed_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("The following files will be overwritten"))
        );
    }

    #[test]
    fn test_share_file_with_conflicts_cancelled() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());
        mock_ui.set_confirm(false);

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

        // Setup: File is already shared (exists in shared storage and is a symlink in repo+main)
        let shared_file = PathBuf::from("/test_root/.shared/github.com/user/repo/test.txt");
        mock_fs.add_file(&shared_file);
        mock_fs.add_symlink(&repo_root.join("test.txt"), &shared_file);

        // Add conflicting worktree with a real file (not a symlink)
        let worktree = PathBuf::from("/test_root/github.com/user/repo+feature");
        mock_fs.add_git_repo(&worktree);
        mock_fs.add_file(&worktree.join("test.txt"));

        let usecase = ShareFilesUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "test.txt");

        // Assert
        assert!(matches!(result, Err(GrmError::UserCancelled)));
    }

    #[test]
    fn test_share_file_not_found() {
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

        // Set current directory to repo root
        mock_fs.set_current_dir(&repo_root);

        let usecase = ShareFilesUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "nonexistent.txt");

        // Assert
        assert!(matches!(result, Err(GrmError::NotFound(_))));
    }
}
