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
                    .filter(|p| !self.fs.is_symlink(p) && p.is_dir())
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
