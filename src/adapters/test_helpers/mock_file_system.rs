//! Mock filesystem for testing
//!
//! Provides an in-memory filesystem simulation with basic operations.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::core::ports::{FileSystem, FileSystemError};

/// Mock entry in the filesystem
#[derive(Debug, Clone)]
struct MockFsEntry {
    is_symlink: bool,
    is_dir: bool,
    target: Option<PathBuf>, // For symlinks
}

/// Mock filesystem for testing
///
/// Provides an in-memory filesystem simulation with basic operations.
pub struct MockFileSystem {
    entries: RefCell<HashMap<PathBuf, MockFsEntry>>,
    home_dir: PathBuf,
    current_dir: RefCell<PathBuf>,
    force_error: RefCell<Option<FileSystemError>>,
}

impl MockFileSystem {
    pub fn new() -> Self {
        let mut entries = HashMap::new();

        // Add root directory by default
        let root_entry = MockFsEntry {
            is_symlink: false,
            is_dir: true,
            target: None,
        };
        entries.insert(PathBuf::from("/"), root_entry);

        Self {
            entries: RefCell::new(entries),
            home_dir: PathBuf::from("/home/testuser"),
            current_dir: RefCell::new(PathBuf::from("/home/testuser/work")),
            force_error: RefCell::new(None),
        }
    }

    /// Add a file to the mock filesystem
    pub fn add_file(&self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        let entry = MockFsEntry {
            is_symlink: false,
            is_dir: false,
            target: None,
        };
        self.entries.borrow_mut().insert(path, entry);
    }

    /// Add a directory to the mock filesystem
    pub fn add_dir(&self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        let entry = MockFsEntry {
            is_symlink: false,
            is_dir: true,
            target: None,
        };
        self.entries.borrow_mut().insert(path, entry);
    }

    /// Add a git repository to the mock filesystem
    pub fn add_git_repo(&self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        self.add_dir(&path);
        let git_path = path.join(".git");
        self.add_dir(&git_path);
    }

    /// Add a symlink to the mock filesystem
    pub fn add_symlink(&self, link: impl AsRef<Path>, target: impl AsRef<Path>) {
        let link = link.as_ref().to_path_buf();
        let target = target.as_ref().to_path_buf();
        let entry = MockFsEntry {
            is_symlink: true,
            is_dir: false,
            target: Some(target),
        };
        self.entries.borrow_mut().insert(link, entry);
    }

    /// Set the current directory for testing
    pub fn set_current_dir(&self, path: impl AsRef<Path>) {
        *self.current_dir.borrow_mut() = path.as_ref().to_path_buf();
    }

    /// Set the home directory for testing
    pub fn set_home_dir(&mut self, path: impl AsRef<Path>) {
        self.home_dir = path.as_ref().to_path_buf();
    }

    /// Inject an error to be returned on the next operation
    pub fn inject_error(&self, error: FileSystemError) {
        *self.force_error.borrow_mut() = Some(error);
    }

    fn check_error(&self) -> Result<(), FileSystemError> {
        if let Some(err) = self.force_error.borrow_mut().take() {
            return Err(err);
        }
        Ok(())
    }
}

impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for MockFileSystem {
    fn exists(&self, path: &Path) -> bool {
        self.entries.borrow().contains_key(path)
    }

    fn is_symlink(&self, path: &Path) -> bool {
        self.entries
            .borrow()
            .get(path)
            .is_some_and(|e| e.is_symlink)
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.entries.borrow().get(path).is_some_and(|e| e.is_dir)
    }

    fn is_git_repository(&self, path: &Path) -> bool {
        let git_path = path.join(".git");
        self.exists(&git_path)
    }

    fn home_dir(&self) -> Result<PathBuf, FileSystemError> {
        self.check_error()?;
        Ok(self.home_dir.clone())
    }

    fn current_dir(&self) -> Result<PathBuf, FileSystemError> {
        self.check_error()?;
        Ok(self.current_dir.borrow().clone())
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        self.check_error()?;

        let entries = self.entries.borrow();

        // Check if the path exists and is a directory
        if !entries.contains_key(path) {
            return Err(FileSystemError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Directory not found",
            )));
        }

        let entry = entries.get(path).unwrap();
        if !entry.is_dir {
            return Err(FileSystemError::Io(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Not a directory",
            )));
        }

        let children: Vec<PathBuf> = entries
            .keys()
            .filter(|p| p.parent() == Some(path))
            .cloned()
            .collect();

        Ok(children)
    }

    fn create_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        self.check_error()?;

        // Create parent directories recursively
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            if !self.exists(&current) {
                self.add_dir(&current);
            }
        }
        Ok(())
    }

    fn create_symlink(&self, target: &Path, link: &Path) -> Result<(), FileSystemError> {
        self.check_error()?;
        self.add_symlink(link, target);
        Ok(())
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        self.check_error()?;

        let entries = self.entries.borrow();
        let entry = entries
            .get(from)
            .ok_or_else(|| {
                FileSystemError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Source not found",
                ))
            })?
            .clone();

        drop(entries);

        if entry.is_dir {
            // Recursive copy for directories
            self.create_dir(to)?;
            let children = self.read_dir(from)?;
            for child in children {
                let name = child.file_name().unwrap();
                let dest_child = to.join(name);
                self.copy(&child, &dest_child)?;
            }
        } else {
            // File copy
            self.entries.borrow_mut().insert(to.to_path_buf(), entry);
        }

        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        self.check_error()?;

        let mut entries = self.entries.borrow_mut();

        // For directories, we need to rename all children as well
        let is_dir = entries.get(from).is_some_and(|e| e.is_dir);

        if is_dir {
            // Collect all paths that start with 'from'
            let to_rename: Vec<(PathBuf, PathBuf)> = entries
                .keys()
                .filter(|p| *p == from || p.starts_with(from))
                .map(|old_path| {
                    let relative = old_path.strip_prefix(from).unwrap();
                    let new_path = to.join(relative);
                    (old_path.clone(), new_path)
                })
                .collect();

            // Remove old entries and insert new ones
            for (old_path, new_path) in to_rename {
                if let Some(entry) = entries.remove(&old_path) {
                    entries.insert(new_path, entry);
                }
            }
        } else {
            // Simple file rename
            let entry = entries.remove(from).ok_or_else(|| {
                FileSystemError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Source not found",
                ))
            })?;
            entries.insert(to.to_path_buf(), entry);
        }

        Ok(())
    }

    fn remove(&self, path: &Path) -> Result<(), FileSystemError> {
        self.check_error()?;

        let mut entries = self.entries.borrow_mut();

        // Remove path and all children
        let to_remove: Vec<PathBuf> = entries
            .keys()
            .filter(|p| *p == path || p.starts_with(path))
            .cloned()
            .collect();

        for p in to_remove {
            entries.remove(&p);
        }

        Ok(())
    }

    fn normalize(&self, path: &Path, base: &Path) -> Result<PathBuf, FileSystemError> {
        self.check_error()?;

        if path.as_os_str().is_empty() {
            return Err(FileSystemError::PathError(
                "Cannot normalize an empty path".into(),
            ));
        }

        let path_str = path.to_string_lossy();

        // Tilde expansion
        if path_str.starts_with('~') {
            let home = self.home_dir()?;
            let without_tilde = path_str.trim_start_matches("~/").trim_start_matches('~');
            return Ok(home.join(without_tilde));
        }

        // Absolute path
        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }

        // Relative path
        Ok(base.join(path))
    }
}
