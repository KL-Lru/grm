use std::fs;
use std::path::{Component, Path, PathBuf, absolute};

use crate::core::ports::{FileSystem, FileSystemError};

#[derive(Debug)]
pub struct UnixFs;

impl UnixFs {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UnixFs {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for UnixFs {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_symlink(&self, path: &Path) -> bool {
        match path.symlink_metadata() {
            Ok(metadata) => metadata.is_symlink(),
            Err(_) => false,
        }
    }

    fn is_git_repository(&self, path: &Path) -> bool {
        let git_path = path.join(".git");
        git_path.exists() && (git_path.is_dir() || git_path.is_file())
    }

    fn current_dir(&self) -> Result<PathBuf, FileSystemError> {
        let dir = std::env::current_dir()?;
        Ok(dir)
    }

    fn home_dir(&self) -> Result<PathBuf, FileSystemError> {
        dirs::home_dir()
            .and_then(|path| absolute(&path).ok())
            .ok_or_else(|| FileSystemError::PathError("Home directory not found".into()))
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let entries = fs::read_dir(path)?;
        let mut paths = Vec::new();

        for entry in entries {
            let entry = entry?;
            paths.push(entry.path());
        }

        Ok(paths)
    }

    fn create_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        fs::create_dir_all(path)?;
        Ok(())
    }

    fn create_symlink(&self, target: &Path, link: &Path) -> Result<(), FileSystemError> {
        std::os::unix::fs::symlink(target, link)?;
        Ok(())
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        if from.is_dir() {
            self.create_dir(to)?;
            for entry in self.read_dir(from)? {
                let file_name = entry
                    .file_name()
                    .ok_or_else(|| FileSystemError::PathError("Invalid filename".into()))?;

                let dest_path = to.join(file_name);
                if entry.is_dir() {
                    self.copy(&entry, &dest_path)?;
                } else {
                    fs::copy(&entry, &dest_path)?;
                }
            }
        } else {
            fs::copy(from, to)?;
        }
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        fs::rename(from, to)?;
        Ok(())
    }

    fn remove(&self, path: &Path) -> Result<(), FileSystemError> {
        if path.is_dir() && !self.is_symlink(path) {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }

        Ok(())
    }

    fn normalize(&self, path: &Path, base: &Path) -> Result<PathBuf, FileSystemError> {
        if path.as_os_str().is_empty() {
            return Err(FileSystemError::PathError(
                "Cannot normalize an empty path".into(),
            ));
        }

        let components = path.components();
        let mut normalized_path = PathBuf::new();
        let mut first = true;

        for component in components {
            match component {
                Component::RootDir => {
                    normalized_path.push(component);
                }
                Component::Normal(stem) if stem == "~" => {
                    normalized_path.clear();
                    let home = self.home_dir()?;
                    let home_components = home.components();
                    for home_comp in home_components {
                        normalized_path.push(home_comp);
                    }
                }
                Component::Normal(_) => {
                    if first {
                        let base_components = base.components();
                        for base_comp in base_components {
                            normalized_path.push(base_comp);
                        }
                    }
                    normalized_path.push(component);
                }
                Component::Prefix(_) | Component::CurDir => {
                    continue;
                }
                Component::ParentDir => {
                    normalized_path.pop();
                }
            }
            first = false;
        }

        Ok(normalized_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();

        assert!(adapter.exists(temp_dir.path()));
        assert!(!adapter.exists(&temp_dir.path().join("nonexistent")));
    }

    #[test]
    fn test_create_dir_all() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let nested_path = temp_dir.path().join("a/b/c");

        adapter.create_dir(&nested_path).unwrap();
        assert!(adapter.exists(&nested_path));
    }

    #[test]
    fn test_is_git_repository() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir(&repo_dir).unwrap();

        assert!(!adapter.is_git_repository(&repo_dir));

        let git_dir = repo_dir.join(".git");
        fs::create_dir(&git_dir).unwrap();
        assert!(adapter.is_git_repository(&repo_dir));
    }

    #[test]
    fn test_is_git_repository_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let repo_dir = temp_dir.path().join("worktree");
        fs::create_dir(&repo_dir).unwrap();

        let git_file = repo_dir.join(".git");
        fs::File::create(&git_file).unwrap();
        assert!(adapter.is_git_repository(&repo_dir));
    }

    #[test]
    fn test_is_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let target = temp_dir.path().join("target");
        fs::File::create(&target).unwrap();

        let link = temp_dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(adapter.is_symlink(&link));
        assert!(!adapter.is_symlink(&target));
    }

    #[test]
    fn test_read_dir() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();

        fs::File::create(temp_dir.path().join("file1.txt")).unwrap();
        fs::File::create(temp_dir.path().join("file2.txt")).unwrap();

        let entries = adapter.read_dir(temp_dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_remove_dir() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let dir_to_remove = temp_dir.path().join("dir");

        adapter.create_dir(&dir_to_remove).unwrap();
        assert!(adapter.exists(&dir_to_remove));

        adapter.remove(&dir_to_remove).unwrap();
        assert!(!adapter.exists(&dir_to_remove));
    }

    #[test]
    fn test_remove_dir_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let dir_to_remove = temp_dir.path().join("dir/nested");

        adapter.create_dir(&dir_to_remove).unwrap();
        fs::File::create(dir_to_remove.join("file.txt")).unwrap();

        let parent = temp_dir.path().join("dir");
        adapter.remove(&parent).unwrap();
        assert!(!adapter.exists(&parent));
    }

    #[test]
    fn test_remove_file() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let file_path = temp_dir.path().join("file.txt");

        fs::File::create(&file_path).unwrap();
        assert!(adapter.exists(&file_path));

        adapter.remove(&file_path).unwrap();
        assert!(!adapter.exists(&file_path));
    }

    #[test]
    fn test_rename() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let from = temp_dir.path().join("from.txt");
        let to = temp_dir.path().join("to.txt");

        fs::File::create(&from).unwrap();
        assert!(adapter.exists(&from));

        adapter.rename(&from, &to).unwrap();
        assert!(!adapter.exists(&from));
        assert!(adapter.exists(&to));
    }

    #[test]
    fn test_normalize_absolute_path() {
        let adapter = UnixFs::new();
        let base = PathBuf::from("/base");
        let path = PathBuf::from("/absolute/path");

        let normalized = adapter.normalize(&path, &base).unwrap();

        assert!(normalized.is_absolute());
        assert_eq!(normalized.to_string_lossy(), "/absolute/path");
    }

    #[test]
    fn test_normalize_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = UnixFs::new();
        let base = temp_dir.path();
        let path = PathBuf::from("relative/path");

        let normalized = adapter.normalize(&path, base).unwrap();

        assert!(normalized.is_absolute());
        assert_eq!(normalized, base.join("relative/path"));
    }

    #[test]
    fn test_normalize_tilde() {
        let adapter = UnixFs::new();
        let base = PathBuf::from("/some/base");
        let path = PathBuf::from("~");

        let normalized = adapter.normalize(&path, &base).unwrap();
        let home = adapter.home_dir().unwrap();

        assert_eq!(normalized, home);
    }

    #[test]
    fn test_normalize_tilde_with_path() {
        let adapter = UnixFs::new();
        let base = PathBuf::from("/some/base");
        let path = PathBuf::from("~/Documents");

        let normalized = adapter.normalize(&path, &base).unwrap();
        let home = adapter.home_dir().unwrap();

        assert_eq!(normalized, home.join("Documents"));
    }

    #[test]
    fn test_normalize_empty_path() {
        let adapter = UnixFs::new();
        let base = PathBuf::from("/base");
        let path = PathBuf::from("");

        let result = adapter.normalize(&path, &base);

        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_with_dots() {
        let adapter = UnixFs::new();
        let base = PathBuf::from("/base");
        let path = PathBuf::from("hoge/../foo/./bar");

        let normalized = adapter.normalize(&path, &base).unwrap();

        assert!(normalized.is_absolute());
        assert_eq!(normalized, PathBuf::from("/base/foo/bar"));
    }

    #[test]
    fn test_normalize_absolute_with_dots() {
        let adapter = UnixFs::new();
        let base = PathBuf::from("/base");
        let path = PathBuf::from("/absolute/hoge/../foo/./bar");

        let normalized = adapter.normalize(&path, &base).unwrap();

        assert_eq!(normalized, PathBuf::from("/absolute/foo/bar"));
    }

    #[test]
    fn test_normalize_tilde_with_dots() {
        let adapter = UnixFs::new();
        let base = PathBuf::from("/base");
        let path = PathBuf::from("~/hoge/../foo/./bar");

        let normalized = adapter.normalize(&path, &base).unwrap();
        let home = adapter.home_dir().unwrap();

        assert_eq!(normalized, home.join("foo/bar"));
    }
}
