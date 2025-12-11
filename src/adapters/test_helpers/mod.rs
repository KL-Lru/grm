//! Test helpers for mocking dependencies in tests
//!
//! This module provides mock implementations of the core ports:
//! - `MockFileSystem`: In-memory filesystem simulation
//! - `MockGitRepository`: Git operations simulation
//! - `MockUserInteraction`: User interaction simulation
//!
//! These mocks are designed to be simple and focused on testing,
//! avoiding unnecessary complexity while providing essential functionality.

mod mock_file_system;
mod mock_git_repository;
mod mock_user_interaction;

pub use mock_file_system::MockFileSystem;
pub use mock_git_repository::MockGitRepository;
pub use mock_user_interaction::MockUserInteraction;
