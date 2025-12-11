use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    core::{RepoInfo, RepoScanner, ports::FileSystem},
    errors::GrmError,
};

pub struct SharedResource {
    repo_info: RepoInfo,
    fs: Arc<dyn FileSystem>,
    scanner: RepoScanner,
    root: PathBuf,
}

impl SharedResource {
    pub fn new(repo_info: RepoInfo, fs: Arc<dyn FileSystem>, root: PathBuf) -> Self {
        let scanner = RepoScanner::new(Arc::clone(&fs));
        Self {
            repo_info,
            fs,
            scanner,
            root,
        }
    }

    /// Check if a file or directory has conflicts in other worktrees
    ///
    /// # Arguments
    /// * `repo_root` - The root directory of the repository
    /// * `relative_path` - Path relative to the repository root
    ///
    /// # Returns
    /// * `Ok(Vec<PathBuf>)` - List of conflicting paths in other worktrees
    /// * `Err(GrmError)` - If an error occurs
    pub fn conflicts(
        &self,
        repo_root: &Path,
        relative_path: &Path,
    ) -> Result<Vec<PathBuf>, GrmError> {
        let current_dir = self.fs.current_dir()?;
        let file = self.fs.normalize(relative_path, &current_dir)?;
        let repo_relative_path = file
            .strip_prefix(repo_root)
            .map_err(|e| GrmError::NotFound(format!("{e}")))?;

        let shared_path = self
            .repo_info
            .build_shared_path(&self.root, repo_relative_path);
        if !self.fs.exists(&shared_path) {
            return Ok(Vec::new());
        }

        let mut conflicts = Vec::new();
        let worktrees = self.scanner.scan_worktrees(&self.root, &self.repo_info)?;
        for worktree in &worktrees {
            let target_in_worktree = worktree.join(repo_relative_path);
            if file == target_in_worktree {
                continue;
            }
            if self.fs.exists(&target_in_worktree) || self.fs.is_symlink(&target_in_worktree) {
                conflicts.push(target_in_worktree);
            }
        }

        Ok(conflicts)
    }

    /// Mount a shared file or directory for new worktrees
    ///
    /// # Arguments
    /// * `repo_root` - The root directory for managed repositories
    pub fn mount(&self, repo_root: &Path) -> Result<(), GrmError> {
        let shared_root = self.repo_info.build_shared_path(&self.root, Path::new(""));

        if !self.fs.exists(&shared_root) {
            return Err(GrmError::NotFound(format!(
                "Shared storage not found at {}",
                shared_root.display()
            )));
        }

        let mut queue = vec![shared_root.clone()];
        while let Some(current_dir) = queue.pop() {
            for entry in self.fs.read_dir(&current_dir)? {
                if self.fs.is_dir(&entry) {
                    self.fs.create_dir(&repo_root.join(&entry))?;
                    queue.push(entry);
                } else {
                    let relative_path = entry
                        .strip_prefix(&shared_root)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                    let target_path = repo_root.join(relative_path);

                    if self.fs.exists(&target_path) || self.fs.is_symlink(&target_path) {
                        self.fs.remove(&target_path)?;
                    }

                    self.fs.create_symlink(&entry, &target_path)?;
                }
            }
        }
        Ok(())
    }

    /// Share a file or directory across all worktrees
    ///
    /// # Arguments
    /// * `repo_root` - The root directory for managed repositories
    /// * `repo_relative_path` - Path relative to the repository root
    ///
    /// # Returns
    /// * `Ok(())` - Successfully shared the resource
    /// * `Err(GrmError)` - If sharing fails
    pub fn share(&self, repo_root: &Path, relative_path: &Path) -> Result<(), GrmError> {
        let current_dir = self.fs.current_dir()?;
        let file = self.fs.normalize(relative_path, &current_dir)?;
        let repo_relative_path = file
            .strip_prefix(repo_root)
            .map_err(|e| GrmError::NotFound(format!("{e}")))?;
        let shared_path = self
            .repo_info
            .build_shared_path(&self.root, repo_relative_path);

        if !self.fs.exists(&file) {
            return Err(GrmError::NotFound(format!(
                "File/Directory not found: {}",
                relative_path.display()
            )));
        }

        // Check if already shared
        if self.fs.is_symlink(&file) {
            return Ok(());
        }

        // Move the file to shared storage if it's not a symlink
        if let Some(parent) = shared_path.parent() {
            self.fs.create_dir(parent)?;
        }

        if self.fs.exists(&shared_path) {
            self.fs.remove(&shared_path)?;
        }

        self.fs.rename(&file, &shared_path)?;
        let worktrees = self.scanner.scan_worktrees(&self.root, &self.repo_info)?;

        // Create symlinks in all worktrees
        for worktree in &worktrees {
            let target_in_worktree = worktree.join(relative_path);

            if self.fs.exists(&target_in_worktree) || self.fs.is_symlink(&target_in_worktree) {
                self.fs.remove(&target_in_worktree)?;
            }

            self.fs.create_symlink(&shared_path, &target_in_worktree)?;
        }

        Ok(())
    }

    /// Unshare a file or directory from worktrees
    ///
    /// # Arguments
    /// * `repo_root` - The root directory for managed repositories
    /// * `repo_relative_path` - Path relative to the repository root
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of symlinks removed
    /// * `Err(GrmError)` - If unsharing fails
    pub fn unshare(&self, repo_root: &Path, relative_path: &Path) -> Result<usize, GrmError> {
        let current_dir = self.fs.current_dir()?;
        let file = self.fs.normalize(relative_path, &current_dir)?;

        let repo_relative_path = file
            .strip_prefix(repo_root)
            .map_err(|e| GrmError::NotFound(format!("{e}")))?;

        let mut removed_count = 0;

        let worktrees = self.scanner.scan_worktrees(&self.root, &self.repo_info)?;
        for worktree in &worktrees {
            let target_in_worktree = worktree.join(repo_relative_path);

            if !self.fs.exists(&target_in_worktree) && !self.fs.is_symlink(&target_in_worktree) {
                continue;
            }

            if self.fs.is_symlink(&target_in_worktree) {
                self.fs.remove(&target_in_worktree)?;
                removed_count += 1;
            }
        }

        Ok(removed_count)
    }

    /// Isolate a shared file/directory in a specific worktree
    ///
    /// # Arguments
    /// * `repo_root` - The root directory for managed repositories
    /// * `repo_relative_path` - Path relative to the repository root
    /// * `worktree_path` - The worktree path where to isolate the file
    ///
    /// # Returns
    /// * `Ok(())` - Successfully isolated the resource
    /// * `Err(GrmError)` - If isolation fails
    pub fn isolate(&self, repo_root: &Path, relative_path: &Path) -> Result<(), GrmError> {
        let current_dir = self.fs.current_dir()?;
        let file = self.fs.normalize(relative_path, &current_dir)?;
        let repo_relative_path = file
            .strip_prefix(repo_root)
            .map_err(|e| GrmError::NotFound(format!("{e}")))?;

        let shared_path = self
            .repo_info
            .build_shared_path(&self.root, repo_relative_path);
        let absolute_target_path = repo_root.join(repo_relative_path);

        if !self.fs.exists(&absolute_target_path) {
            return Err(GrmError::NotFound(format!(
                "File/Directory not found: {}",
                repo_relative_path.display()
            )));
        }

        if !self.fs.is_symlink(&absolute_target_path) {
            return Ok(());
        }

        if !self.fs.exists(&shared_path) {
            return Err(GrmError::NotFound(format!(
                "Shared storage not found at {}",
                shared_path.display()
            )));
        }

        self.fs.remove(&absolute_target_path)?;
        self.fs.copy(&shared_path, &absolute_target_path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers::MockFileSystem;

    fn setup() -> (Arc<MockFileSystem>, RepoInfo, PathBuf) {
        let fs = Arc::new(MockFileSystem::new());
        let repo_info = RepoInfo::from_url("https://github.com/user/repo").unwrap();
        let root = PathBuf::from("/grm");
        fs.add_dir(&root);
        (fs, repo_info, root)
    }

    #[test]
    fn test_share_file_success() {
        // 目的: ファイル共有の基本動作
        // 検証: ファイルが共有ストレージに移動し、各ワークツリーにシンボリックリンクが作成される

        let (fs, repo_info, root) = setup();

        // ワークツリーとファイルの準備
        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo+main"));
        fs.add_git_repo(&root.join("github.com/user/repo+feature"));

        let repo_root = root.join("github.com/user/repo+main");
        fs.add_file(&repo_root.join("config.json"));
        fs.set_current_dir(&repo_root);

        let shared = SharedResource::new(repo_info, fs.clone(), root.clone());
        let result = shared.share(&repo_root, Path::new("config.json"));

        assert!(result.is_ok(), "share failed: {:?}", result.err());

        // 共有ストレージにファイルが存在
        let shared_path = root.join(".shared/github.com/user/repo/config.json");
        assert!(fs.exists(&shared_path));

        // 各ワークツリーにシンボリックリンクが作成される
        assert!(fs.is_symlink(&repo_root.join("config.json")));
        assert!(fs.is_symlink(&root.join("github.com/user/repo+feature/config.json")));
    }

    #[test]
    fn test_share_directory_success() {
        // 目的: ディレクトリ共有
        // 検証: ディレクトリ全体が共有される

        let (fs, repo_info, root) = setup();

        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo+main"));

        let repo_root = root.join("github.com/user/repo+main");
        fs.add_dir(&repo_root.join("shared_dir"));
        fs.add_file(&repo_root.join("shared_dir/file.txt"));
        fs.set_current_dir(&repo_root);

        let shared = SharedResource::new(repo_info, fs.clone(), root.clone());
        let result = shared.share(&repo_root, Path::new("shared_dir"));

        assert!(result.is_ok());

        let shared_path = root.join(".shared/github.com/user/repo/shared_dir");
        assert!(fs.exists(&shared_path));
    }

    #[test]
    fn test_unshare_success() {
        // 目的: シンボリックリンク削除
        // 検証: 削除数が正しく返される

        let (fs, repo_info, root) = setup();

        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo+main"));
        fs.add_git_repo(&root.join("github.com/user/repo+feature"));

        let repo_root = root.join("github.com/user/repo+main");
        let shared_file = root.join(".shared/github.com/user/repo/config.json");
        fs.add_dir(&root.join(".shared"));
        fs.add_dir(&root.join(".shared/github.com"));
        fs.add_dir(&root.join(".shared/github.com/user"));
        fs.add_dir(&root.join(".shared/github.com/user/repo"));
        fs.add_file(&shared_file);

        // シンボリックリンクを作成
        fs.add_symlink(&repo_root.join("config.json"), &shared_file);
        fs.add_symlink(&root.join("github.com/user/repo+feature/config.json"), &shared_file);
        fs.set_current_dir(&repo_root);

        let shared = SharedResource::new(repo_info, fs.clone(), root.clone());
        let result = shared.unshare(&repo_root, Path::new("config.json"));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        // シンボリックリンクが削除されている
        assert!(!fs.exists(&repo_root.join("config.json")));
        assert!(!fs.exists(&root.join("github.com/user/repo+feature/config.json")));
    }

    #[test]
    fn test_isolate_success() {
        // 目的: シンボリックリンクを実体ファイルに置換
        // 検証: シンボリックリンクが削除され、実体ファイルがコピーされる

        let (fs, repo_info, root) = setup();

        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo+main"));

        let repo_root = root.join("github.com/user/repo+main");
        let shared_file = root.join(".shared/github.com/user/repo/config.json");
        fs.add_dir(&root.join(".shared"));
        fs.add_dir(&root.join(".shared/github.com"));
        fs.add_dir(&root.join(".shared/github.com/user"));
        fs.add_dir(&root.join(".shared/github.com/user/repo"));
        fs.add_file(&shared_file);
        fs.add_symlink(&repo_root.join("config.json"), &shared_file);
        fs.set_current_dir(&repo_root);

        let shared = SharedResource::new(repo_info, fs.clone(), root.clone());
        let result = shared.isolate(&repo_root, Path::new("config.json"));

        assert!(result.is_ok());

        // シンボリックリンクではなく実体ファイルになっている
        assert!(!fs.is_symlink(&repo_root.join("config.json")));
        assert!(fs.exists(&repo_root.join("config.json")));
    }

    #[test]
    fn test_conflicts_detection() {
        // 目的: 他のワークツリーとの競合検出
        // 検証: 競合するファイルが正しく検出される

        let (fs, repo_info, root) = setup();

        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo+main"));
        fs.add_git_repo(&root.join("github.com/user/repo+feature"));

        let repo_root = root.join("github.com/user/repo+main");
        let shared_file = root.join(".shared/github.com/user/repo/config.json");
        fs.add_dir(&root.join(".shared"));
        fs.add_dir(&root.join(".shared/github.com"));
        fs.add_dir(&root.join(".shared/github.com/user"));
        fs.add_dir(&root.join(".shared/github.com/user/repo"));
        fs.add_file(&shared_file);
        fs.add_file(&repo_root.join("config.json"));
        fs.add_file(&root.join("github.com/user/repo+feature/config.json"));
        fs.set_current_dir(&repo_root);

        let shared = SharedResource::new(repo_info, fs.clone(), root.clone());
        let result = shared.conflicts(&repo_root, Path::new("config.json"));

        assert!(result.is_ok());
        let conflicts = result.unwrap();

        assert_eq!(conflicts.len(), 1);
        assert!(conflicts.contains(&root.join("github.com/user/repo+feature/config.json")));
    }

    #[test]
    fn test_mount_success() {
        // 目的: 共有ストレージマウント
        // 検証: 共有ファイルがワークツリーにシンボリックリンクとして作成される

        let (fs, repo_info, root) = setup();

        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo+new"));

        let shared_root = root.join(".shared/github.com/user/repo");
        fs.add_dir(&root.join(".shared"));
        fs.add_dir(&root.join(".shared/github.com"));
        fs.add_dir(&root.join(".shared/github.com/user"));
        fs.add_dir(&shared_root);
        fs.add_file(&shared_root.join("config.json"));

        let repo_root = root.join("github.com/user/repo+new");

        let shared = SharedResource::new(repo_info, fs.clone(), root.clone());
        let result = shared.mount(&repo_root);

        assert!(result.is_ok());

        // シンボリックリンクが作成されている
        assert!(fs.is_symlink(&repo_root.join("config.json")));
    }

    #[test]
    fn test_share_file_not_found() {
        // 目的: 存在しないファイルのエラー
        // 検証: NotFoundエラーが返される

        let (fs, repo_info, root) = setup();

        fs.add_dir(&root.join("github.com"));
        fs.add_dir(&root.join("github.com/user"));
        fs.add_git_repo(&root.join("github.com/user/repo+main"));

        let repo_root = root.join("github.com/user/repo+main");
        fs.set_current_dir(&repo_root);

        let shared = SharedResource::new(repo_info, fs.clone(), root.clone());
        let result = shared.share(&repo_root, Path::new("nonexistent.txt"));

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GrmError::NotFound(_)));
    }
}
