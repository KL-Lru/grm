use std::path::PathBuf;

use crate::configs::ConfigError;

/// Trait for configuration providers
///
/// Each provider represents a source of configuration (environment variables,
/// config files, defaults, etc.) and can attempt to load the root path.
///
/// Providers are executed in priority order until one successfully returns a value.
pub trait ConfigProvider {
    /// Attempt to load the root path from this configuration source
    ///
    /// # Returns
    ///
    /// - `Ok(Some(path))`: Configuration found and successfully parsed
    /// - `Ok(None)`: Configuration source does not exist (try next provider)
    /// - `Err(e)`: Configuration exists but failed to parse (stop immediately)
    fn load_root(&self) -> Result<Option<PathBuf>, ConfigError>;
}
