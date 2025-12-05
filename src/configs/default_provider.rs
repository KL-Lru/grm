use std::path::PathBuf;

use crate::configs::{ConfigError, provider::ConfigProvider};
use crate::utils::path;

/// Provider for the default configuration value
///
/// Always returns `~/grm` as the root directory.
/// This provider should be last in the priority chain as a fallback.
pub struct DefaultProvider;

impl ConfigProvider for DefaultProvider {
    fn load_root(&self) -> Result<Option<PathBuf>, ConfigError> {
        let home = path::require_home_dir()?;
        Ok(Some(home.join("grm")))
    }
}
