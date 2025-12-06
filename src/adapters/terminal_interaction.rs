use std::io::{self, BufRead, Write};

use crate::adapters::ports::{InteractionError, UserInteraction};

pub struct TerminalInteraction;

impl TerminalInteraction {
    pub fn new() -> Self {
        Self
    }

    fn confirm_stream<R, W>(
        read: &mut R,
        write: &mut W,
        message: &str,
    ) -> Result<bool, InteractionError>
    where
        R: BufRead,
        W: Write,
    {
        write!(write, "{message} [y/N]: ")?;
        write.flush()?;

        let mut input = String::new();
        read.read_line(&mut input)?;

        let answer = input.trim().to_lowercase();
        Ok(answer.starts_with('y'))
    }
}

impl Default for TerminalInteraction {
    fn default() -> Self {
        Self::new()
    }
}

impl UserInteraction for TerminalInteraction {
    fn confirm(&self, message: &str) -> Result<bool, InteractionError> {
        Self::confirm_stream(&mut io::stdin().lock(), &mut io::stdout(), message)
    }

    fn print(&self, message: &str) {
        println!("{message}");
    }

    fn print_error(&self, message: &str) {
        eprintln!("{message}");
    }
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

        let result = TerminalInteraction::confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(result.unwrap());
        assert_eq!(String::from_utf8(writer).unwrap(), "Continue? [y/N]: ");
    }

    #[test]
    fn test_confirm_yes_full() {
        let input = b"Yes\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = TerminalInteraction::confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(result.unwrap());
    }

    #[test]
    fn test_confirm_no() {
        let input = b"n\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = TerminalInteraction::confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(!result.unwrap());
    }

    #[test]
    fn test_confirm_empty() {
        let input = b"\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = TerminalInteraction::confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(!result.unwrap());
    }

    #[test]
    fn test_confirm_weird_input() {
        let input = b"what\n";
        let mut reader = Cursor::new(&input[..]);
        let mut writer = Vec::new();

        let result = TerminalInteraction::confirm_stream(&mut reader, &mut writer, "Continue?");
        assert!(!result.unwrap());
    }
}
