//! Mock user interaction for testing
//!
//! Provides a mock implementation of user interaction for testing.

use std::sync::Mutex;

use crate::core::ports::{InteractionError, UserInteraction};

/// Mock user interaction for testing
pub struct MockUserInteraction {
    confirm_responses: Mutex<Vec<bool>>,
    printed_messages: Mutex<Vec<String>>,
    error_messages: Mutex<Vec<String>>,
}

impl UserInteraction for MockUserInteraction {
    fn confirm(&self, _message: &str) -> Result<bool, InteractionError> {
        let mut responses = self.confirm_responses.lock().unwrap();

        if let Some(response) = responses.pop() {
            Ok(response)
        } else {
            Ok(false)
        }
    }

    fn print(&self, message: &str) {
        self.printed_messages.lock().unwrap().push(message.to_string());
    }

    fn print_error(&self, message: &str) {
        self.error_messages.lock().unwrap().push(message.to_string());
    }
}

impl MockUserInteraction {
    pub fn new() -> Self {
        Self {
            confirm_responses: Mutex::new(Vec::new()),
            printed_messages: Mutex::new(Vec::new()),
            error_messages: Mutex::new(Vec::new()),
        }
    }

    /// Set the next confirm response
    pub fn set_confirm(&self, response: bool) {
        self.confirm_responses.lock().unwrap().push(response);
    }

    /// Get printed messages (for assertions)
    pub fn get_printed_messages(&self) -> Vec<String> {
        self.printed_messages.lock().unwrap().clone()
    }

    /// Check if a message was printed
    pub fn has_printed(&self, expected: &str) -> bool {
        self.printed_messages
            .lock().unwrap()
            .iter()
            .any(|msg| msg.contains(expected))
    }
}

impl Default for MockUserInteraction {
    fn default() -> Self {
        Self::new()
    }
}
