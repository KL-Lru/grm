use std::io::{self, BufRead, Write};

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
/// ```no_run
/// use grm::utils::prompt;
/// if prompt::confirm("Delete this file?").unwrap() {
///     // proceed with deletion
/// }
/// ```
pub fn confirm(message: &str) -> Result<bool, GrmError> {
    confirm_stream(&mut io::stdin().lock(), &mut io::stdout(), message)
}

fn confirm_stream<R, W>(read: &mut R, write: &mut W, message: &str) -> Result<bool, GrmError>
where
    R: BufRead,
    W: Write,
{
    write!(write, "{message} [y/N]: ").map_err(GrmError::Io)?;
    write.flush().map_err(GrmError::Io)?;

    let mut input = String::new();
    read.read_line(&mut input).map_err(GrmError::Io)?;

    let answer = input.trim().to_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_confirm_yes() {
        let input = b"y\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(result.unwrap());
        assert_eq!(String::from_utf8(writer).unwrap(), "Continue? [y/N]: ");
    }

    #[test]
    fn test_confirm_yes_full() {
        let input = b"Yes\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(result.unwrap());
    }

    #[test]
    fn test_confirm_no() {
        let input = b"n\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(!result.unwrap());
    }

    #[test]
    fn test_confirm_empty() {
        let input = b"\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(!result.unwrap());
    }

    #[test]
    fn test_confirm_weird_input() {
        let input = b"what\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(!result.unwrap());
    }
}
