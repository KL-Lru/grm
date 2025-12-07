use std::path::PathBuf;
use std::sync::Arc;

use crate::core::ports::FileSystem;
use crate::configs::{ConfigError, provider::ConfigProvider};

/// Provider for the default configuration value
///
/// Always returns `~/grm` as the root directory.
/// This provider should be last in the priority chain as a fallback.
pub struct DefaultProvider {
    fs: Arc<dyn FileSystem>,
}

impl DefaultProvider {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }
}

impl ConfigProvider for DefaultProvider {
    fn load_root(&self) -> Result<Option<PathBuf>, ConfigError> {
        let home = self.fs.home_dir()?;
        Ok(Some(home.join("grm")))
    }
}
