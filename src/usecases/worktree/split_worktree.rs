use std::sync::Arc;

use crate::configs::Config;
use crate::core::RepoInfo;
use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::core::shared_resource::SharedResource;
use crate::errors::GrmError;

pub struct SplitWorktreeUseCase {
    git: Arc<dyn GitRepository>,
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl SplitWorktreeUseCase {
    pub fn new(
        git: Arc<dyn GitRepository>,
        fs: Arc<dyn FileSystem>,
        ui: Arc<dyn UserInteraction>,
    ) -> Self {
        Self { git, fs, ui }
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

        let dest_path = repo_info.build_repo_path(config.root(), branch);

        if self.fs.exists(&dest_path) {
            return Err(GrmError::AlreadyExists(dest_path.display().to_string()));
        }

        if let Some(parent) = dest_path.parent() {
            self.fs.create_dir(parent)?;
        }

        let already_exists = self.git.local_branch_exists(branch)?
            || self.git.remote_branch_exists(&remote_url, branch)?;

        self.git.add_worktree(&dest_path, branch, !already_exists)?;

        self.ui.print(&dest_path.display().to_string());

        let shared_resource =
            SharedResource::new(repo_info, Arc::clone(&self.fs), config.root().to_path_buf());
        shared_resource.mount(&repo_root)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockFileSystem, MockGitRepository, MockUserInteraction};
    use std::path::PathBuf;

    #[test]
    fn test_split_worktree_new_branch() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        let repo_root = PathBuf::from("/test_root/github.com/user/repo+main");
        mock_git.set_repo_root(&repo_root);
        mock_git.set_remote_url(&repo_root, "https://github.com/user/repo");

        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo(&repo_root);
        mock_fs.add_dir("/test_root/.shared");
        mock_fs.add_dir("/test_root/.shared/github.com");
        mock_fs.add_dir("/test_root/.shared/github.com/user");
        mock_fs.add_dir("/test_root/.shared/github.com/user/repo");

        let usecase = SplitWorktreeUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "feature");

        // Assert
        if let Err(ref e) = result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok(), "Failed with error: {:?}", result.err());
        let worktrees = mock_git.get_worktrees();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(
            worktrees[0],
            PathBuf::from("/test_root/github.com/user/repo+feature")
        );
    }

    #[test]
    fn test_split_worktree_existing_branch() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        let repo_root = PathBuf::from("/test_root/github.com/user/repo+main");
        mock_git.set_repo_root(&repo_root);
        mock_git.set_remote_url(&repo_root, "https://github.com/user/repo");
        mock_git.add_local_branch("develop");

        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo(&repo_root);
        mock_fs.add_dir("/test_root/.shared");
        mock_fs.add_dir("/test_root/.shared/github.com");
        mock_fs.add_dir("/test_root/.shared/github.com/user");
        mock_fs.add_dir("/test_root/.shared/github.com/user/repo");

        let usecase = SplitWorktreeUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "develop");

        // Assert
        assert!(result.is_ok());
        let worktrees = mock_git.get_worktrees();
        assert_eq!(worktrees.len(), 1);
    }

    #[test]
    fn test_split_worktree_already_exists() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        let repo_root = PathBuf::from("/test_root/github.com/user/repo+main");
        mock_git.set_repo_root(&repo_root);
        mock_git.set_remote_url(&repo_root, "https://github.com/user/repo");

        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo(&repo_root);
        mock_fs.add_dir("/test_root/.shared");
        mock_fs.add_dir("/test_root/.shared/github.com");
        mock_fs.add_dir("/test_root/.shared/github.com/user");
        mock_fs.add_dir("/test_root/.shared/github.com/user/repo");
        mock_fs.add_git_repo("/test_root/github.com/user/repo+feature");

        let usecase = SplitWorktreeUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "feature");

        // Assert
        assert!(matches!(result, Err(GrmError::AlreadyExists(_))));
    }

    #[test]
    fn test_split_worktree_not_in_repo() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        // Don't set repo_root or remote_url - simulates not being in a repository

        let usecase = SplitWorktreeUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "feature");

        // Assert
        assert!(matches!(result, Err(GrmError::NotInManagedRepository)));
    }

    #[test]
    fn test_split_worktree_remote_branch() {
        // Arrange
        let mock_git = Arc::new(MockGitRepository::new());
        let mock_fs = Arc::new(MockFileSystem::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        let repo_root = PathBuf::from("/test_root/github.com/user/repo+main");
        let remote_url = "https://github.com/user/repo";
        mock_git.set_repo_root(&repo_root);
        mock_git.set_remote_url(&repo_root, remote_url);
        mock_git.add_remote_branch(remote_url, "release");

        mock_fs.add_dir("/test_root");
        mock_fs.add_dir("/test_root/github.com");
        mock_fs.add_dir("/test_root/github.com/user");
        mock_fs.add_git_repo(&repo_root);
        mock_fs.add_dir("/test_root/.shared");
        mock_fs.add_dir("/test_root/.shared/github.com");
        mock_fs.add_dir("/test_root/.shared/github.com/user");
        mock_fs.add_dir("/test_root/.shared/github.com/user/repo");

        let usecase = SplitWorktreeUseCase::new(mock_git.clone(), mock_fs.clone(), mock_ui.clone());

        let config = Config {
            root: PathBuf::from("/test_root"),
        };

        // Act
        let result = usecase.execute(&config, "release");

        // Assert
        assert!(result.is_ok());
        let worktrees = mock_git.get_worktrees();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(
            worktrees[0],
            PathBuf::from("/test_root/github.com/user/repo+release")
        );
    }
}
