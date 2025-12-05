use std::path::PathBuf;

use serde::Deserialize;

use crate::configs::{ConfigError, provider::ConfigProvider};
use crate::utils::path;

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
pub struct GrmrcProvider;

impl ConfigProvider for GrmrcProvider {
    fn load_root(&self) -> Result<Option<PathBuf>, ConfigError> {
        let home = path::require_home_dir()?;

        let grmrc_path = home.join(".grmrc");

        // If file doesn't exist, return None to try next provider
        let content = match std::fs::read_to_string(&grmrc_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ConfigError::Io(format!("Failed to read .grmrc: {e}"))),
        };

        // Parse TOML - any parse error should stop immediately
        let parsed: GrmrcFile = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(format!("Failed to parse .grmrc: {e}")))?;

        // Normalize the path
        let normalized = path::normalize_path(&parsed.root)?;

        Ok(Some(normalized))
    }
}
