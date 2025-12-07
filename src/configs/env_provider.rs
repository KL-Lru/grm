use std::path::PathBuf;
use std::sync::Arc;

use crate::core::ports::FileSystem;
use crate::configs::{ConfigError, provider::ConfigProvider};

/// Provider for environment variable configuration
///
/// Reads the `GRM_ROOT` environment variable and normalizes the path.
pub struct EnvProvider {
    fs: Arc<dyn FileSystem>,
}

impl EnvProvider {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }
}

impl ConfigProvider for EnvProvider {
    fn load_root(&self) -> Result<Option<PathBuf>, ConfigError> {
        match std::env::var("GRM_ROOT") {
            Ok(path_str) => {
                let home = self.fs.home_dir()?;
                let path = std::path::Path::new(&path_str);
                let normalized = self.fs.normalize(path, &home)?;
                Ok(Some(normalized))
            }
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(e) => Err(ConfigError::Env(e.to_string())),
        }
    }
}
