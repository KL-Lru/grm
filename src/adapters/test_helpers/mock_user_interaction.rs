//! Mock user interaction for testing
//!
//! Provides a mock implementation of user interaction for testing.

use std::cell::RefCell;

use crate::core::ports::{InteractionError, UserInteraction};

/// Mock user interaction for testing
pub struct MockUserInteraction {
    confirm_responses: RefCell<Vec<bool>>,
    printed_messages: RefCell<Vec<String>>,
    error_messages: RefCell<Vec<String>>,
}

impl UserInteraction for MockUserInteraction {
    fn confirm(&self, _message: &str) -> Result<bool, InteractionError> {
        let mut responses = self.confirm_responses.borrow_mut();

        if let Some(response) = responses.pop() {
            Ok(response)
        } else {
            Ok(false)
        }
    }

    fn print(&self, message: &str) {
        self.printed_messages.borrow_mut().push(message.to_string());
    }

    fn print_error(&self, message: &str) {
        self.error_messages.borrow_mut().push(message.to_string());
    }
}

impl MockUserInteraction {
    pub fn new() -> Self {
        Self {
            confirm_responses: RefCell::new(Vec::new()),
            printed_messages: RefCell::new(Vec::new()),
            error_messages: RefCell::new(Vec::new()),
        }
    }

    /// Set the next confirm response
    pub fn set_confirm(&self, response: bool) {
        self.confirm_responses.borrow_mut().push(response);
    }

    /// Get printed messages (for assertions)
    pub fn get_printed_messages(&self) -> Vec<String> {
        self.printed_messages.borrow().clone()
    }

    /// Get error messages (for assertions)
    pub fn get_error_messages(&self) -> Vec<String> {
        self.error_messages.borrow().clone()
    }

    /// Check if a message was printed
    pub fn has_printed(&self, expected: &str) -> bool {
        self.printed_messages
            .borrow()
            .iter()
            .any(|msg| msg.contains(expected))
    }
}

impl Default for MockUserInteraction {
    fn default() -> Self {
        Self::new()
    }
}
