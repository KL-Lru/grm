use std::path::PathBuf;
use std::sync::Arc;

use crate::core::ports::FileSystem;
use crate::configs::{ConfigError, provider::ConfigProvider};

const GIT_CONFIG_ROOT_KEY: &str = "grm.root";

/// Provider for ~/.gitconfig configuration
///
/// Reads the `grm.root` key from the `[grm]` section in `~/.gitconfig`.
///
/// Example configuration:
///
/// ```ini
/// [grm]
///     root = /path/to/root
/// ```
pub struct GitConfigProvider {
    fs: Arc<dyn FileSystem>,
}

impl GitConfigProvider {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }
}

impl ConfigProvider for GitConfigProvider {
    fn load_root(&self) -> Result<Option<PathBuf>, ConfigError> {
        // Try to open the default git config
        let config = match git2::Config::open_default() {
            Ok(c) => c,
            Err(e) => {
                // If .gitconfig doesn't exist, skip to next provider
                if e.code() == git2::ErrorCode::NotFound {
                    return Ok(None);
                }
                return Err(ConfigError::GitConfig(e.to_string()));
            }
        };

        // Try to get the grm.root key
        let root_str = match config.get_string(GIT_CONFIG_ROOT_KEY) {
            Ok(s) => s,
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                // Key doesn't exist, skip to next provider
                return Ok(None);
            }
            Err(e) => {
                return Err(ConfigError::GitConfig(e.to_string()));
            }
        };

        // Normalize the path
        let home = self.fs.home_dir()?;
        let path = std::path::Path::new(&root_str);
        let normalized = self.fs.normalize(path, &home)?;

        Ok(Some(normalized))
    }
}
