use std::path::PathBuf;
use std::sync::Arc;

use crate::configs::Config;
use crate::core::RepoInfo;
use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::errors::GrmError;

pub struct InitRepositoryUseCase {
    git: Arc<dyn GitRepository>,
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl InitRepositoryUseCase {
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

        let branch_name = branch.unwrap_or("main");

        let dest_path = repo_info.build_repo_path(config.root(), branch_name);

        if self.fs.exists(&dest_path) {
            return Err(GrmError::AlreadyExists(dest_path.display().to_string()));
        }

        if let Some(parent) = dest_path.parent() {
            self.fs.create_dir(parent)?;
        }

        self.git.init_repository(&dest_path, branch_name)?;

        self.ui.print(&format!(
            "Repository initialized at: {}",
            dest_path.display()
        ));

        Ok(dest_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockFileSystem, MockGitRepository, MockUserInteraction};
    use crate::core::ports::GitError;

    fn setup() -> (
        Arc<MockGitRepository>,
        Arc<MockFileSystem>,
        Arc<MockUserInteraction>,
        Config,
    ) {
        let git = Arc::new(MockGitRepository::new());
        let fs = Arc::new(MockFileSystem::new());
        let ui = Arc::new(MockUserInteraction::new());

        let root = PathBuf::from("/home/testuser/grm");
        fs.add_dir(&root);
        let config = Config { root };

        (git, fs, ui, config)
    }

    #[test]
    fn test_init_success_with_default_branch() {
        // 目的: デフォルトブランチ(main)で初期化成功
        // 検証: 正しいパスに初期化され、メッセージが表示される

        let (git, fs, ui, config) = setup();
        let usecase = InitRepositoryUseCase::new(git.clone(), fs.clone(), ui.clone());

        let url = "https://github.com/user/repo.git";

        let result = usecase.execute(&config, url, None);

        assert!(result.is_ok(), "init failed: {:?}", result.err());
        let dest = result.unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/home/testuser/grm/github.com/user/repo+main")
        );

        let initialized = git.get_initialized_repos();
        assert_eq!(initialized.len(), 1);
        assert_eq!(initialized[0].0, dest);
        assert_eq!(initialized[0].1, "main");

        assert!(ui.has_printed("Repository initialized at"));
    }

    #[test]
    fn test_init_success_with_specified_branch() {
        // 目的: ブランチ指定で初期化成功
        // 検証: 指定したブランチで初期化される

        let (git, fs, ui, config) = setup();
        let usecase = InitRepositoryUseCase::new(git.clone(), fs.clone(), ui.clone());

        let url = "git@github.com:user/repo.git";

        let result = usecase.execute(&config, url, Some("feature"));

        assert!(result.is_ok(), "init failed: {:?}", result.err());
        let dest = result.unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/home/testuser/grm/github.com/user/repo+feature")
        );

        let initialized = git.get_initialized_repos();
        assert_eq!(initialized.len(), 1);
        assert_eq!(initialized[0].1, "feature");
    }

    #[test]
    fn test_init_already_exists() {
        // 目的: 既存のパスへの初期化を防ぐ
        // 検証: AlreadyExistsエラーが返される

        let (git, fs, ui, config) = setup();
        let usecase = InitRepositoryUseCase::new(git.clone(), fs.clone(), ui.clone());

        let url = "https://github.com/user/repo.git";

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
    fn test_init_invalid_url() {
        // 目的: 不正なURL形式を検出
        // 検証: ParseFailedエラーが返される

        let (git, fs, ui, config) = setup();
        let usecase = InitRepositoryUseCase::new(git, fs, ui);

        let result = usecase.execute(&config, "invalid-url", None);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GrmError::ParseFailed(_)));
    }

    #[test]
    fn test_init_git_failure() {
        // 目的: Git操作失敗時のエラーハンドリング
        // 検証: GitErrorが適切に伝播される

        let (git, fs, ui, config) = setup();
        let usecase = InitRepositoryUseCase::new(git.clone(), fs, ui);

        let url = "https://github.com/user/repo.git";
        git.inject_error(GitError::Execution("Permission denied".into()));

        let result = usecase.execute(&config, url, None);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GrmError::Git(_)));
    }
}
