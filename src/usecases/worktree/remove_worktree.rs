use std::sync::Arc;

use crate::core::ports::{GitRepository, UserInteraction};
use crate::configs::Config;
use crate::core::RepoInfo;
use crate::errors::GrmError;

pub struct RemoveWorktreeUseCase {
    git: Arc<dyn GitRepository>,
    ui: Arc<dyn UserInteraction>,
}

impl RemoveWorktreeUseCase {
    pub fn new(git: Arc<dyn GitRepository>, ui: Arc<dyn UserInteraction>) -> Self {
        Self { git, ui }
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

        let worktree_path = repo_info.build_repo_path(config.root(), branch);

        if !worktree_path.exists() {
            return Err(GrmError::NotFound(format!(
                "Worktree does not exist: {}",
                worktree_path.display()
            )));
        }

        self.git
            .remove_worktree(&worktree_path)
            .map_err(GrmError::Git)?;

        self.ui
            .print(&format!("Removed worktree: {}", worktree_path.display()));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockGitRepository, MockUserInteraction};
    use std::path::PathBuf;
    use std::fs;

    #[test]
    fn test_remove_worktree_success() {
        // Arrange
        let temp_dir = tempfile::tempdir().unwrap();
        let test_root = temp_dir.path().to_path_buf();

        let mock_git = Arc::new(MockGitRepository::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        let repo_root = test_root.join("github.com/user/repo+main");
        mock_git.set_repo_root(&repo_root);
        mock_git.set_remote_url(&repo_root, "https://github.com/user/repo");

        let worktree_path = test_root.join("github.com/user/repo+feature");
        fs::create_dir_all(&worktree_path).unwrap();

        let usecase = RemoveWorktreeUseCase::new(mock_git.clone(), mock_ui.clone());

        let config = Config {
            root: test_root.clone(),
        };

        // Act
        let result = usecase.execute(&config, "feature");

        // Assert
        assert!(result.is_ok());
        let messages = mock_ui.get_printed_messages();
        assert!(messages
            .iter()
            .any(|m| m.contains("Removed worktree")));
    }

    #[test]
    fn test_remove_worktree_not_exists() {
        // Arrange
        let temp_dir = tempfile::tempdir().unwrap();
        let test_root = temp_dir.path().to_path_buf();

        let mock_git = Arc::new(MockGitRepository::new());
        let mock_ui = Arc::new(MockUserInteraction::new());

        let repo_root = test_root.join("github.com/user/repo+main");
        mock_git.set_repo_root(&repo_root);
        mock_git.set_remote_url(&repo_root, "https://github.com/user/repo");

        let usecase = RemoveWorktreeUseCase::new(mock_git.clone(), mock_ui.clone());

        let config = Config {
            root: test_root.clone(),
        };

        // Act
        let result = usecase.execute(&config, "nonexistent");

        // Assert
        assert!(matches!(result, Err(GrmError::NotFound(_))));
    }
}
