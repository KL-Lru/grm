use thiserror::Error;

use crate::{
    configs::ConfigError,
    core::ports::{FileSystemError, GitError, InteractionError},
    core::repo_info::RepositoryError,
    core::repo_scanner::ScanError,
};

#[derive(Debug, Error)]
pub enum GrmError {
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("URL / Path error: {0}")]
    ParseFailed(#[from] RepositoryError),

    #[error("File system error: {0}")]
    FileSystem(#[from] FileSystemError),

    #[error("Interaction error: {0}")]
    Interaction(#[from] InteractionError),

    #[error("Scan error: {0}")]
    Scan(#[from] ScanError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path already exists: {0}")]
    AlreadyExists(String),

    #[error("No repositories found for URL: {url}\nSearched in: {searched_path}")]
    UnmanagedRepository { url: String, searched_path: String },

    #[error("Operation cancelled by user")]
    UserCancelled,

    #[error("Not in a managed git repository")]
    NotInManagedRepository,

    #[error("Resource not found: {0}")]
    NotFound(String),
}
