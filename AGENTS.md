# AGENTS.md - Guidelines for AI Coding Agents

## Project Overview

**git_sv** is a Rust CLI/TUI application for visualizing git graphs in the terminal. Built with ratatui, crossterm, and git2.

---

## Build Commands

```bash
# Build the project
cargo build

# Build for release (optimized binary)
cargo build --release

# Run the application (interactive TUI)
cargo run

# Run with arguments
cargo run -- log -n 10
cargo run -- --path /path/to/repo

# Run in non-interactive mode (print commit log)
cargo run -- log

# Build with profiling feature
cargo build --features profiling

# Build with vendored OpenSSL
cargo build --features vendored-ssl
```

---

## Test Commands

```bash
# Run all tests
cargo test

# Run a specific test by name
cargo test test_name

# Run tests in a specific module
cargo test module_name::

# Run with output visible (for debugging)
cargo test -- --nocapture

# Run tests matching a pattern
cargo test pattern

# Run integration tests only
cargo test --test integration_test

# Run tests and generate coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html --output-dir coverage
```

---

## Lint & Format Commands

```bash
# Format code (standard Rust style)
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run Clippy lints
cargo clippy

# Run Clippy with all features and warnings as errors
cargo clippy --all-features -- -D warnings

# Check for common mistakes
cargo check
```

---

## Code Style Guidelines

### Imports
- Group imports: std, external crates, then internal modules
- Use `use crate::` for internal imports
- Re-export public items in `mod.rs` files

```rust
// Standard library
use std::io::{self, Stdout};

// External crates
use ratatui::{backend::CrosstermBackend, Terminal};
use anyhow::Result;
use clap::{Parser, Subcommand};

// Internal modules
use crate::error::Result;
use crate::git::GitRepo;
```

### Naming Conventions
- **Types**: PascalCase (`GitRepo`, `CommitNode`, `AppAction`)
- **Functions/Variables**: snake_case (`build_graph`, `selected_index`)
- **Constants**: UPPER_SNAKE_CASE (`MAX_COMMITS`)
- **Modules**: snake_case (`graph_view`, `status_view`)
- **Error types**: PascalCase ending with `Error` (`GitSvError`)

### Types & Error Handling
- Use `anyhow::Result` for application-level errors (main.rs)
- Use `thiserror` for custom error enums
- Propagate errors with `?` operator
- Use `IoErrorContext` trait for adding context to I/O errors

```rust
use crate::error::{GitSvError, IoErrorContext, Result};

// With context
let file = File::open(path).with_context(|| format!("Failed to open {}", path))?;
```

### Comments & Documentation
- Code comments in **French** (follow existing convention)
- Use `///` for public API documentation
- Use `//` for inline comments
- Use `//!` for module-level documentation

```rust
/// Rafraîchit les données depuis le repository git.
pub fn refresh(&mut self) -> Result {
    // Réajuster la sélection si nécessaire.
}
```

### Structs & Enums
- Derive common traits: `Debug`, `Clone`, `PartialEq` when applicable
- Use named fields over tuple structs for clarity
- Use `pub` visibility modifier explicitly

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    Quit,
    MoveUp,
    MoveDown,
}
```

### Functions
- Keep functions focused and under 50 lines when possible
- Use early returns to reduce nesting
- Document panics and errors in doc comments

### Pattern Matching
- Use exhaustive matching, prefer `_` over `if/else` chains
- Group similar cases with `|` operator

```rust
match action {
    AppAction::Quit => self.should_quit = true,
    AppAction::MoveUp | AppAction::MoveDown => self.update_selection(action),
    _ => {}
}
```

---

## Testing Guidelines

- Add unit tests in the same file under `#[cfg(test)]` module
- Use integration tests in `tests/` directory
- Use `tempfile` and `git2` to create test repositories
- Use `insta` for snapshot testing when appropriate
- Mock git operations when possible
- Test error cases, not just happy paths

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{create_test_repo, commit_file};

    #[test]
    fn test_feature() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "test.txt", "content", "Test commit");
        // Assert
    }
}
```

---

## Module Organization

```
src/
├── main.rs          # Entry point, CLI parsing
├── app.rs           # App orchestration
├── error.rs         # Error types and Result alias
├── error_display.rs # Error display utilities
├── terminal.rs      # Terminal setup/teardown
├── watcher.rs       # File system watching
├── git/             # Git operations
│   ├── mod.rs       # Re-exports GitRepo
│   ├── repo.rs      # Repository wrapper
│   ├── graph.rs     # Graph building
│   ├── commit.rs    # Commit info
│   ├── branch.rs    # Branch operations
│   └── ...
├── handler/         # Event handlers
│   ├── mod.rs
│   ├── dispatcher.rs
│   └── ...
├── state/           # Application state
│   ├── mod.rs       # AppState
│   ├── action.rs    # AppAction enums
│   └── ...
├── ui/              # UI rendering
│   ├── mod.rs
│   ├── graph_view.rs
│   └── ...
├── utils/           # Utilities
└── test_utils/      # Test helpers
```

---

## Pre-commit Checklist

Before committing changes:
1. `cargo build` succeeds
2. `cargo test` passes
3. `cargo fmt` applied
4. `cargo clippy` shows no warnings
5. Code follows naming conventions
6. Comments are in French (follow existing convention)
