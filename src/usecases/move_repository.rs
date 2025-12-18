use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::configs::Config;
use crate::core::RepoInfo;
use crate::core::ports::{FileSystem, GitRepository, UserInteraction};
use crate::errors::GrmError;

pub struct MoveRepositoryUseCase {
    git: Arc<dyn GitRepository>,
    fs: Arc<dyn FileSystem>,
    ui: Arc<dyn UserInteraction>,
}

impl MoveRepositoryUseCase {
    pub fn new(
        git: Arc<dyn GitRepository>,
        fs: Arc<dyn FileSystem>,
        ui: Arc<dyn UserInteraction>,
    ) -> Self {
        Self { git, fs, ui }
    }

    pub fn execute(&self, config: &Config, source: &str) -> Result<PathBuf, GrmError> {
        // 1. Normalize source path (supports ~, relative, absolute)
        let cwd = self.fs.current_dir()?;
        let source_path = self.fs.normalize(Path::new(source), &cwd)?;

        // 2. Check if source is a git repository
        if !self.fs.is_git_repository(&source_path) {
            return Err(GrmError::NotAGitRepository(
                source_path.display().to_string(),
            ));
        }

        // 3. Check if source is NOT already under GRM management
        if source_path.starts_with(config.root()) {
            return Err(GrmError::AlreadyManaged(source_path.display().to_string()));
        }

        // 4. Get remote origin URL
        let remote_url = self
            .git
            .get_remote_url(&source_path)
            .map_err(|_| GrmError::NoRemoteOrigin(source_path.display().to_string()))?;

        // 5. Parse remote URL to RepoInfo
        let repo_info = RepoInfo::from_url(&remote_url)?;

        // 6. Get current branch
        let branch = self.git.get_current_branch(&source_path)?;

        // 7. Build target path
        let target_path = repo_info.build_repo_path(config.root(), &branch);

        // 8. Check if target already exists
        if self.fs.exists(&target_path) {
            return Err(GrmError::AlreadyExists(target_path.display().to_string()));
        }

        // 9. Create parent directory
        if let Some(parent) = target_path.parent() {
            self.fs.create_dir(parent)?;
        }

        // 10. Move repository
        self.fs.rename(&source_path, &target_path)?;

        // 11. Print success message
        self.ui
            .print(&format!("Repository moved to: {}", target_path.display()));

        Ok(target_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::{MockFileSystem, MockGitRepository, MockUserInteraction};

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
    fn test_move_success() {
        // 目的: Gitリポジトリを正常にGRM管理下に移動
        // 検証: 正しいパスに移動され、メッセージが表示される

        let (git, fs, ui, config) = setup();
        let usecase = MoveRepositoryUseCase::new(git.clone(), fs.clone(), ui.clone());

        let source = PathBuf::from("/home/testuser/projects/myrepo");
        fs.add_dir(&source);
        fs.add_git_repo(&source);

        git.set_remote_url(&source, "https://github.com/user/repo.git");
        git.set_current_branch(&source, "main");

        let result = usecase.execute(&config, "/home/testuser/projects/myrepo");

        assert!(result.is_ok(), "move failed: {:?}", result.err());
        let dest = result.unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/home/testuser/grm/github.com/user/repo+main")
        );

        assert!(ui.has_printed("Repository moved to"));
    }

    #[test]
    fn test_move_not_a_git_repo() {
        // 目的: Git以外のディレクトリの移動を拒否
        // 検証: NotAGitRepositoryエラーが返される

        let (git, fs, ui, config) = setup();
        let usecase = MoveRepositoryUseCase::new(git, fs.clone(), ui);

        let source = PathBuf::from("/home/testuser/projects/notgit");
        fs.add_dir(&source);

        let result = usecase.execute(&config, "/home/testuser/projects/notgit");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GrmError::NotAGitRepository(_)
        ));
    }

    #[test]
    fn test_move_already_managed() {
        // 目的: 既にGRM管理下のリポジトリの移動を拒否
        // 検証: AlreadyManagedエラーが返される

        let (git, fs, ui, config) = setup();
        let usecase = MoveRepositoryUseCase::new(git, fs.clone(), ui);

        let source = PathBuf::from("/home/testuser/grm/github.com/user/repo+main");
        fs.add_dir(&source);
        fs.add_git_repo(&source);

        let result = usecase.execute(&config, "/home/testuser/grm/github.com/user/repo+main");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GrmError::AlreadyManaged(_)));
    }

    #[test]
    fn test_move_no_remote_origin() {
        // 目的: remote originがないリポジトリの移動を拒否
        // 検証: NoRemoteOriginエラーが返される

        let (git, fs, ui, config) = setup();
        let usecase = MoveRepositoryUseCase::new(git, fs.clone(), ui);

        let source = PathBuf::from("/home/testuser/projects/localrepo");
        fs.add_dir(&source);
        fs.add_git_repo(&source);

        // remote origin未設定 (set_remote_urlを呼ばない)

        let result = usecase.execute(&config, "/home/testuser/projects/localrepo");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GrmError::NoRemoteOrigin(_)));
    }

    #[test]
    fn test_move_target_already_exists() {
        // 目的: ターゲットパスが既に存在する場合を拒否
        // 検証: AlreadyExistsエラーが返される

        let (git, fs, ui, config) = setup();
        let usecase = MoveRepositoryUseCase::new(git.clone(), fs.clone(), ui);

        let source = PathBuf::from("/home/testuser/projects/myrepo");
        fs.add_dir(&source);
        fs.add_git_repo(&source);

        git.set_remote_url(&source, "https://github.com/user/repo.git");
        git.set_current_branch(&source, "main");

        let target = PathBuf::from("/home/testuser/grm/github.com/user/repo+main");
        fs.add_dir(&target);

        let result = usecase.execute(&config, "/home/testuser/projects/myrepo");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GrmError::AlreadyExists(_)));
    }

    #[test]
    fn test_move_with_hierarchical_branch() {
        // 目的: feature/testのような階層ブランチの処理
        // 検証: 正しくパスが構築される

        let (git, fs, ui, config) = setup();
        let usecase = MoveRepositoryUseCase::new(git.clone(), fs.clone(), ui.clone());

        let source = PathBuf::from("/home/testuser/projects/myrepo");
        fs.add_dir(&source);
        fs.add_git_repo(&source);

        git.set_remote_url(&source, "git@github.com:user/repo.git");
        git.set_current_branch(&source, "feature/test");

        let result = usecase.execute(&config, "/home/testuser/projects/myrepo");

        assert!(result.is_ok(), "move failed: {:?}", result.err());
        let dest = result.unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/home/testuser/grm/github.com/user/repo+feature/test")
        );
    }
}
