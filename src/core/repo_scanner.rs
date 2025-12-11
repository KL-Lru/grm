use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::RepoInfo;
use crate::core::ports::FileSystem;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("IO error during scanning: {0}")]
    Io(String),
}

pub struct RepoScanner {
    fs: Arc<dyn FileSystem>,
}

impl RepoScanner {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }
}

impl RepoScanner {
    pub fn scan_repositories(&self, root: &Path) -> Result<Vec<PathBuf>, ScanError> {
        let mut repos = Vec::new();
        match self.fs.read_dir(root) {
            Ok(entries) => {
                let dirs = entries
                    .into_iter()
                    .filter(|p| !self.fs.is_symlink(p) && self.fs.is_dir(p))
                    .collect::<Vec<_>>();

                for dir in dirs {
                    if self.fs.is_git_repository(&dir) {
                        repos.push(dir);
                    } else {
                        let sub_repos = self.scan_repositories(&dir)?;
                        repos.extend(sub_repos);
                    }
                }

                Ok(repos)
            }
            Err(e) => Err(ScanError::Io(e.to_string())),
        }
    }

    pub fn scan_worktrees(
        &self,
        root: &Path,
        repo_info: &RepoInfo,
    ) -> Result<Vec<PathBuf>, ScanError> {
        let mut repos = Vec::new();
        let all_repos = self.scan_repositories(root)?;

        // root/host/user/repo+
        let repo_prefix = repo_info
            .build_repo_path(root, "")
            .to_string_lossy()
            .to_string();

        for repo_path in all_repos {
            if repo_path.to_string_lossy().starts_with(&repo_prefix) {
                repos.push(repo_path);
            }
        }

        Ok(repos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::MockFileSystem;

    #[test]
    fn test_scan_repositories_flat_structure() {
        // 目的: フラットな構造のスキャン
        // 検証: ルート直下の複数のリポジトリが全て検出される

        let fs = Arc::new(MockFileSystem::new());
        let root = PathBuf::from("/grm");

        fs.add_dir(&root);
        fs.add_git_repo(&root.join("repo1"));
        fs.add_git_repo(&root.join("repo2"));
        fs.add_git_repo(&root.join("repo3"));

        let scanner = RepoScanner::new(fs);
        let result = scanner.scan_repositories(&root);

        assert!(result.is_ok());
        let mut repos = result.unwrap();
        repos.sort();

        assert_eq!(repos.len(), 3);
        assert!(repos.contains(&root.join("repo1")));
        assert!(repos.contains(&root.join("repo2")));
        assert!(repos.contains(&root.join("repo3")));
    }

    #[test]
    fn test_scan_repositories_nested_structure() {
        // 目的: ネストした構造の再帰スキャン
        // 検証: 深い階層のリポジトリも検出される

        let fs = Arc::new(MockFileSystem::new());
        let root = PathBuf::from("/grm");

        fs.add_dir(&root);
        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo1"));
        fs.add_git_repo(&root.join("github.com/user/repo2"));

        fs.add_dir(&root.join("gitlab.com"));
        fs.add_dir(&root.join("gitlab.com/org"));
        fs.add_git_repo(&root.join("gitlab.com/org/project"));

        let scanner = RepoScanner::new(fs);
        let result = scanner.scan_repositories(&root);

        assert!(result.is_ok());
        let repos = result.unwrap();

        assert_eq!(repos.len(), 3);
        assert!(repos.contains(&root.join("github.com/user/repo1")));
        assert!(repos.contains(&root.join("github.com/user/repo2")));
        assert!(repos.contains(&root.join("gitlab.com/org/project")));
    }

    #[test]
    fn test_scan_repositories_skip_symlinks() {
        // 目的: シンボリックリンクのスキップ
        // 検証: シンボリックリンクは検出されず、実体のみ検出される

        let fs = Arc::new(MockFileSystem::new());
        let root = PathBuf::from("/grm");

        fs.add_dir(&root);
        fs.add_git_repo(&root.join("real_repo"));
        fs.add_symlink(&root.join("link_to_repo"), &root.join("real_repo"));

        let scanner = RepoScanner::new(fs);
        let result = scanner.scan_repositories(&root);

        assert!(result.is_ok());
        let repos = result.unwrap();

        assert_eq!(repos.len(), 1);
        assert!(repos.contains(&root.join("real_repo")));
        assert!(!repos.contains(&root.join("link_to_repo")));
    }

    #[test]
    fn test_scan_worktrees_filter_by_repo_info() {
        // 目的: 特定リポジトリのワークツリーのみをフィルタリング
        // 検証: 指定したリポジトリに属するワークツリーだけが返される

        let fs = Arc::new(MockFileSystem::new());
        let root = PathBuf::from("/grm");

        fs.add_dir(&root);
        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo1+main"));
        fs.add_git_repo(&root.join("github.com/user/repo1+feature"));
        fs.add_git_repo(&root.join("github.com/user/repo2+main"));

        let scanner = RepoScanner::new(fs);

        // repo1 のワークツリーのみを取得
        let repo_info = RepoInfo::from_url("https://github.com/user/repo1").unwrap();
        let result = scanner.scan_worktrees(&root, &repo_info);

        assert!(result.is_ok());
        let worktrees = result.unwrap();

        assert_eq!(worktrees.len(), 2);
        assert!(worktrees.contains(&root.join("github.com/user/repo1+main")));
        assert!(worktrees.contains(&root.join("github.com/user/repo1+feature")));
        assert!(!worktrees.contains(&root.join("github.com/user/repo2+main")));
    }

    #[test]
    fn test_scan_repositories_empty() {
        // 目的: リポジトリなしの場合
        // 検証: 空のベクタが返される

        let fs = Arc::new(MockFileSystem::new());
        let root = PathBuf::from("/grm");

        fs.add_dir(&root);
        fs.add_dir(&root.join("empty_dir"));

        let scanner = RepoScanner::new(fs);
        let result = scanner.scan_repositories(&root);

        assert!(result.is_ok());
        let repos = result.unwrap();

        assert_eq!(repos.len(), 0);
    }
}
