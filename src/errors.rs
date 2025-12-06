use thiserror::Error;

use crate::{configs::ConfigError, utils::git::GitError, utils::git_url::UrlError};

#[derive(Debug, Error)]
pub enum GrmError {
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("URL error: {0}")]
    Url(#[from] UrlError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Directory already exists: {0}")]
    AlreadyManaged(String),

    #[error("No repositories found for URL: {url}\nSearched in: {searched_path}")]
    UnmanagedRepository { url: String, searched_path: String },

    #[error("Operation cancelled by user")]
    UserCancelled,
}
