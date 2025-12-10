# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Grm is a Git CLI tool that manages multiple Git repositories and worktrees in a structured directory layout. It provides commands to clone, list, remove repositories, and advanced worktree management features like sharing files/directories between worktrees.

## Build and Development Commands

```bash
# Build the project
cargo build

# Build with optimizations
cargo build --release

# Run the application
cargo run

# Run with arguments (example: clone a repo)
cargo run -- clone https://github.com/user/repo.git

# Check code without building
cargo check

# Run tests
cargo nextest run

# Run a specific test
cargo nextest run <test_name>

# Run tests with backtrace
RUST_BACKTRACE=1 cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Architecture

This project follows Clean Architecture / Hexagonal Architecture principles with clear separation of concerns:

### Layers

1. **Commands** (`src/commands.rs`)
   - CLI interface using clap's derive API
   - Parses user input and coordinates use case execution
   - Main entry point: `Cli::execute()`

2. **Use Cases** (`src/usecases/`)
   - Core logic layer, independent of external dependencies
   - Each command has a corresponding use case (e.g., `CloneRepositoryUseCase`, `SplitWorktreeUseCase`)
   - Use cases depend on port traits, not concrete implementations

3. **Core** (`src/core/`)
   - `RepoInfo` - Repository information parsing and path handling
   - `RepoScanner` - Scans managed repositories
   - `shared_resource` - Shared resource management for worktrees
   - `ports/` - Port traits defining contracts:
     - `GitRepository` - Git operations interface
     - `FileSystem` - File system operations interface
     - `UserInteraction` - User interaction interface (prompts, output)

4. **Adapters** (`src/adapters/`)
   - Concrete implementations of port traits:
     - `GitCli` - Git operations via git2 library
     - `UnixFs` - File system operations
     - `TerminalInteraction` - Terminal I/O
   - `test_helpers/` - Mock implementations for testing

5. **Container** (`src/container.rs`)
   - Dependency injection container
   - `AppContainer::new_production()` wires up concrete implementations

6. **Configuration** (`src/configs.rs`)
   - Loads configuration from multiple sources (priority order):
     1. `GRM_ROOT` environment variable
     2. `~/.grmrc` (TOML format)
     3. `~/.gitconfig` ([grm] section)
     4. Default: `~/grm`
   - Uses chain of responsibility pattern with internal provider implementations

### Key Design Patterns

- **Ports and Adapters**: Core domain depends only on port traits, making it testable and framework-independent
- **Dependency Injection**: `AppContainer` constructs and wires dependencies
- **Use Case Pattern**: Each command maps to a dedicated use case class
- **Error Handling**: Uses `thiserror` for domain-specific error types (`GrmError`, `ConfigError`, etc.)

### Testing Strategy

- Unit tests use mock implementations from `src/adapters/test_helpers/`
- Temporary directories created with `tempfile` crate for integration tests
- Port traits enable testing use cases without real Git operations or file I/O

## Key Concepts

### Repository Path Structure

Repositories are cloned to: `$(grm root)/<host>/<user>/<repo>+<branch>`
- Example: `~/grm/github.com/user/repo+main`

### Shared Resources

Shared files/directories are stored in: `$(grm root)/.shared/<host>/<user>/<repo>/<path>`
- Worktrees reference shared resources via symbolic links
- New worktrees automatically inherit existing shared resources

## Lints

- `unsafe_code = "forbid"` - No unsafe code allowed
- `clippy::all = "warn"` - All clippy warnings enabled
- `clippy::pedantic = "warn"` - Pedantic clippy warnings enabled
