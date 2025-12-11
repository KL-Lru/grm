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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockFileSystem, MockGitRepository, MockUserInteraction};
    use crate::core::ports::GitError;

    fn setup() -> (Arc<MockGitRepository>, Arc<MockFileSystem>, Arc<MockUserInteraction>, Config) {
        let git = Arc::new(MockGitRepository::new());
        let fs = Arc::new(MockFileSystem::new());
        let ui = Arc::new(MockUserInteraction::new());

        let root = PathBuf::from("/home/testuser/grm");
        fs.add_dir(&root);
        let config = Config { root };

        (git, fs, ui, config)
    }

    #[test]
    fn test_clone_success_with_default_branch() {
        // 目的: HTTPSリポジトリをデフォルトブランチでクローン成功
        // 検証: 正しいパスにクローンされ、メッセージが表示される

        let (git, fs, ui, config) = setup();
        let usecase = CloneRepositoryUseCase::new(git.clone(), fs.clone(), ui.clone());

        let url = "https://github.com/user/repo.git";
        git.set_default_branch(url, "main");

        let result = usecase.execute(&config, url, None);

        assert!(result.is_ok(), "clone failed: {:?}", result.err());
        let dest = result.unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/home/testuser/grm/github.com/user/repo+main")
        );

        let cloned = git.get_cloned_repos();
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned[0].0, url);
        assert_eq!(cloned[0].1, dest);

        assert!(ui.has_printed("Repository cloned to"));
    }

    #[test]
    fn test_clone_success_with_specified_branch() {
        // 目的: ブランチ指定でクローン成功
        // 検証: 指定したブランチでクローンされる

        let (git, fs, ui, config) = setup();
        let usecase = CloneRepositoryUseCase::new(git.clone(), fs.clone(), ui.clone());

        let url = "git@github.com:user/repo.git";

        let result = usecase.execute(&config, url, Some("feature/test"));

        assert!(result.is_ok(), "clone failed: {:?}", result.err());
        let dest = result.unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/home/testuser/grm/github.com/user/repo+feature/test")
        );

        let cloned = git.get_cloned_repos();
        assert_eq!(cloned.len(), 1);
    }

    #[test]
    fn test_clone_already_exists() {
        // 目的: 既存のパスへのクローンを防ぐ
        // 検証: AlreadyExistsエラーが返される

        let (git, fs, ui, config) = setup();
        let usecase = CloneRepositoryUseCase::new(git.clone(), fs.clone(), ui.clone());

        let url = "https://github.com/user/repo.git";
        git.set_default_branch(url, "main");

        let dest_path = PathBuf::from("/home/testuser/grm/github.com/user/repo+main");
        fs.add_dir(&dest_path);

        let result = usecase.execute(&config, url, None);

        assert!(result.is_err());
        match result.unwrap_err() {
            GrmError::AlreadyExists(path) => {
                assert!(path.contains("repo+main"));
            }
            _ => panic!("Expected AlreadyExists error"),
        }
    }

    #[test]
    fn test_clone_invalid_url() {
        // 目的: 不正なURL形式を検出
        // 検証: ParseFailedエラーが返される

        let (git, fs, ui, config) = setup();
        let usecase = CloneRepositoryUseCase::new(git, fs, ui);

        let result = usecase.execute(&config, "invalid-url", None);

        assert!(result.is_err());
        // Invalid URL should result in ParseFailed error
        assert!(matches!(result.unwrap_err(), GrmError::ParseFailed(_)));
    }

    #[test]
    fn test_clone_git_failure() {
        // 目的: Git操作失敗時のエラーハンドリング
        // 検証: GitErrorが適切に伝播される

        let (git, fs, ui, config) = setup();
        let usecase = CloneRepositoryUseCase::new(git.clone(), fs, ui);

        let url = "https://github.com/user/repo.git";
        git.set_default_branch(url, "main");
        git.inject_error(GitError::Execution("Network error".into()));

        let result = usecase.execute(&config, url, None);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GrmError::Git(_)));
    }
}

