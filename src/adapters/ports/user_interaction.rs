use std::io;
use thiserror::Error;

/// Errors that can occur during user interaction
#[derive(Debug, Error)]
pub enum InteractionError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Interface for user interaction operations
///
/// This trait abstracts user interaction in CLI applications, allowing for
/// different implementations (e.g., terminal, mock for testing).
pub trait UserInteraction: Send + Sync {
    /// Prompts the user for confirmation
    ///
    /// # Arguments
    /// * `message` - The confirmation message to display (e.g., "Delete this file?")
    ///
    /// # Returns
    /// * `Ok(true)` - User confirmed (yes)
    /// * `Ok(false)` - User declined (no)
    /// * `Err` - Failed to read user input
    fn confirm(&self, message: &str) -> Result<bool, InteractionError>;

    /// Prints a message to the user
    ///
    /// # Arguments
    /// * `message` - The message to display
    fn print(&self, message: &str);

    /// Prints an error message to the user
    ///
    /// # Arguments
    /// * `message` - The error message to display
    fn print_error(&self, message: &str);
}
