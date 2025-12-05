use std::path::PathBuf;

use crate::configs::{ConfigError, provider::ConfigProvider};
use crate::utils::path;

/// Provider for environment variable configuration
///
/// Reads the `GRM_ROOT` environment variable and normalizes the path.
pub struct EnvProvider;

impl ConfigProvider for EnvProvider {
    fn load_root(&self) -> Result<Option<PathBuf>, ConfigError> {
        match std::env::var("GRM_ROOT") {
            Ok(path_str) => {
                let normalized = path::normalize_path(&path_str)?;
                Ok(Some(normalized))
            }
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(e) => Err(ConfigError::Env(e.to_string())),
        }
    }
}
