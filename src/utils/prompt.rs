use std::io::{self, Write};

use crate::errors::GrmError;

/// Prompt the user for yes/no confirmation
///
/// Displays a message and waits for user input.
/// Accepts "y", "Y", "yes", "Yes", "YES" as confirmation.
/// Any other input (including empty) is treated as rejection.
///
/// # Arguments
/// * `message` - The message to display before the prompt
///
/// # Returns
/// * `Ok(true)` if user confirmed
/// * `Ok(false)` if user rejected
/// * `Err` if IO error occurred
///
/// # Examples
/// ```
/// if confirm("Delete this file?")? {
///     // proceed with deletion
/// }
/// ```
pub fn confirm(message: &str) -> Result<bool, GrmError> {
    print!("{message} [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let answer = input.trim().to_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}
