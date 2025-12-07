use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;

use crate::core::ports::FileSystem;
use crate::configs::{ConfigError, provider::ConfigProvider};

/// TOML structure for .grmrc file
#[derive(Debug, Deserialize)]
struct GrmrcFile {
    root: String,
}

/// Provider for ~/.grmrc configuration file
///
/// Reads and parses a TOML file at `~/.grmrc` with the following format:
///
/// ```toml
/// root = "/path/to/root"
/// ```
pub struct GrmrcProvider {
    fs: Arc<dyn FileSystem>,
}

impl GrmrcProvider {
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }
}

impl ConfigProvider for GrmrcProvider {
    fn load_root(&self) -> Result<Option<PathBuf>, ConfigError> {
        let home = self.fs.home_dir()?;

        let grmrc_path = home.join(".grmrc");

        // If file doesn't exist, return None to try next provider
        let content = match std::fs::read_to_string(&grmrc_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ConfigError::Io(format!("Failed to read .grmrc: {e}"))),
        };

        // Parse TOML
        let parsed: GrmrcFile = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(format!("Failed to parse .grmrc: {e}")))?;

        // Normalize the path
        let path = std::path::Path::new(&parsed.root);
        let normalized = self.fs.normalize(path, &home)?;

        Ok(Some(normalized))
    }
}
