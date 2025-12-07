//! Configuration management for Grm
//!
//! This module provides a unified `Config` struct that loads settings from
//! multiple sources in priority order. The internal provider implementations
//! are private to enforce the standard configuration loading pattern.
//!
//! # Configuration Priority
//!
//! 1. Environment variable `GRM_ROOT`
//! 2. `~/.grmrc` (TOML format)
//! 3. `~/.gitconfig` ([grm] section)
//! 4. Default: `~/grm`

// Internal provider implementations (private)
mod default_provider;
mod env_provider;
mod gitconfig_provider;
mod grmrc_provider;
pub(crate) mod provider; // Available within crate for testing

use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

use crate::core::ports::FileSystemError;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to parse config: {0}")]
    Parse(String),

    #[error("Failed to read config: {0}")]
    Io(String),

    #[error("Git config error: {0}")]
    GitConfig(String),

    #[error("Environment variable error: {0}")]
    Env(String),

    #[error("File system error: {0}")]
    FileSystem(#[from] FileSystemError),
}

/// Grm configuration manager
#[derive(Debug, Clone)]
pub struct Config {
    /// Root directory for repository management
    pub root: PathBuf,
}

impl Config {
    /// Load configuration and build Grm Config
    ///
    /// Priority order:
    /// 1. ENV ``GRM_ROOT``
    /// 2. ~/.grmrc (TOML format)
    /// 3. ~/.gitconfig ([grm] section)
    /// 4. Default: ~/grm
    pub fn load() -> Result<Self, ConfigError> {
        use crate::adapters::unix_fs::UnixFs;
        use provider::ConfigProvider;

        let fs = Arc::new(UnixFs::new());

        // Build the provider chain in priority order
        let providers: Vec<Box<dyn ConfigProvider>> = vec![
            Box::new(env_provider::EnvProvider::new(fs.clone())),
            Box::new(grmrc_provider::GrmrcProvider::new(fs.clone())),
            Box::new(gitconfig_provider::GitConfigProvider::new(fs.clone())),
            Box::new(default_provider::DefaultProvider::new(fs.clone())),
        ];

        // Try each provider in order until one returns a value
        for provider in providers {
            match provider.load_root() {
                Ok(Some(root)) => {
                    // Found a configuration, return it
                    return Ok(Config { root });
                }
                Ok(None) => {}
                Err(e) => {
                    // Parse error - stop immediately
                    return Err(e);
                }
            }
        }

        // DefaultProvider should always return Some, so this is unreachable
        unreachable!("DefaultProvider should always return a value")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
