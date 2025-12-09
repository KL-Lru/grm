use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FileSystemError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path error: {0}")]
    PathError(String),
}

pub trait FileSystem: Send + Sync {
    /// Check if a path exists
    ///
    /// # Arguments
    /// * `path` - The path to check
    ///
    /// # Returns
    /// * `true` if the path exists (file or directory)
    /// * `false` if the path does not exist
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a symbolic link
    ///
    /// # Arguments
    /// * `path` - The path to check
    ///
    /// # Returns
    /// * `true` if the path is a symbolic link
    /// * `false` if the path is not a symbolic link or does not exist
    fn is_symlink(&self, path: &Path) -> bool;

    /// Check if a path is a git repository
    ///
    /// # Arguments
    /// * `path` - The path to check
    ///
    /// # Returns
    /// * `true` if the path contains a `.git` directory or file (for worktrees)
    /// * `false` otherwise
    fn is_git_repository(&self, path: &Path) -> bool;

    /// Get the home directory path
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - The absolute path to the user's home directory
    /// * `Err` - If the home directory cannot be determined
    fn home_dir(&self) -> Result<PathBuf, FileSystemError>;

    /// Get the current working directory
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - The absolute path to the current working directory
    /// * `Err` - If the current directory cannot be determined
    fn current_dir(&self) -> Result<PathBuf, FileSystemError>;

    /// Read a directory and return all entries
    ///
    /// # Arguments
    /// * `path` - The directory path to read
    ///
    /// # Returns
    /// * `Ok(Vec<PathBuf>)` - List of all entries in the directory
    /// * `Err` - If the directory cannot be read
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError>;

    /// Create a directory and all necessary parent directories
    ///
    /// # Arguments
    /// * `path` - The directory path to create
    ///
    /// # Returns
    /// * `Ok(())` - Directory created successfully
    /// * `Err` - If the directory cannot be created
    fn create_dir(&self, path: &Path) -> Result<(), FileSystemError>;

    /// Create a symbolic link
    ///
    /// # Arguments
    /// * `target` - The target path the symlink points to
    /// * `link` - The path where the symlink will be created
    ///
    /// # Returns
    /// * `Ok(())` - Symlink created successfully
    /// * `Err` - If the symlink cannot be created
    fn create_symlink(&self, target: &Path, link: &Path) -> Result<(), FileSystemError>;

    /// Copy a file or directory
    ///
    /// # Arguments
    /// * `from` - The source path
    /// * `to` - The destination path
    ///
    /// # Returns
    /// * `Ok(())` - Copied successfully
    /// * `Err` - If the copy operation fails
    fn copy(&self, from: &Path, to: &Path) -> Result<(), FileSystemError>;

    /// Rename or move a file or directory
    ///
    /// # Arguments
    /// * `from` - The source path
    /// * `to` - The destination path
    ///
    /// # Returns
    /// * `Ok(())` - Renamed successfully
    /// * `Err` - If the operation fails
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FileSystemError>;

    /// Remove a directory and all its contents recursively
    ///
    /// # Arguments
    /// * `path` - The directory or file path to remove
    ///
    /// # Returns
    /// * `Ok(())` - removed successfully
    /// * `Err` - If the directory / file cannot be removed
    fn remove(&self, path: &Path) -> Result<(), FileSystemError>;

    /// Normalize a path to an absolute ``PathBuf``
    ///
    /// # Arguments
    /// * `path` - The path to normalize (supports `~` expansion)
    /// * `base` - Base directory for resolving relative paths
    ///
    /// # Path resolution rules
    /// - `~` or `~/path`: Expanded to home directory (base parameter is ignored)
    /// - `/absolute/path`: Used as-is (base parameter is ignored)
    /// - `relative/path`: Resolved from the base directory
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - The normalized absolute path
    /// * `Err` - If the path cannot be normalized
    fn normalize(&self, path: &Path, base: &Path) -> Result<PathBuf, FileSystemError>;
}
