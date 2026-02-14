# AGENTS.md - Guidelines for AI Coding Agents

## Project Overview

**git_sv** is a Rust CLI/TUI application for visualizing git graphs in the terminal. Built with ratatui, crossterm, and git2.

---

## Build Commands

```bash
# Build the project
cargo build

# Build for release
cargo build --release

# Run the application
cargo run

# Run with arguments
cargo run -- log -n 10
cargo run -- --path /path/to/repo
```

---

## Test Commands

```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run tests in a specific module
cargo test module_name::

# Run with output visible
cargo test -- --nocapture

# Run tests matching a pattern
cargo test pattern
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
- Use `anyhow::Result` for application-level errors
- Use `thiserror` for custom error enums
- Propagate errors with `?` operator
- Avoid unwrap/expect in production code

```rust
// Custom errors
#[derive(Debug, Error)]
pub enum GitSvError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GitSvError>;
```

### Comments & Documentation
- Code comments in **French** (follow existing convention)
- Use `///` for public API documentation
- Use `//` for inline comments
- Document complex algorithms and business logic

```rust
/// Rafraîchit les données depuis le repository git.
pub fn refresh(&mut self) -> Result {
    // Réajuster la sélection si nécessaire.
}
```

### Structs & Enums
- Derive common traits: `Debug`, `Clone`, `PartialEq` when applicable
- Use named fields over tuple structs for clarity

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    Quit,
    MoveUp,
    MoveDown,
}

pub struct App {
    pub repo: GitRepo,
    pub selected_index: usize,
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

### Module Organization
```
src/
├── main.rs          # Entry point, CLI parsing
├── app.rs           # App state and event loop
├── error.rs         # Error types
├── git/             # Git operations
│   ├── mod.rs       # Re-exports
│   ├── repo.rs      # Repository wrapper
│   └── ...
└── ui/              # UI rendering
    ├── mod.rs
    └── ...
```

---

## Dependencies

Key crates (check Cargo.toml for versions):
- `ratatui` - TUI framework
- `crossterm` - Terminal backend
- `git2` - Git operations
- `clap` - CLI parsing (derive feature)
- `anyhow` - Error handling
- `thiserror` - Custom errors
- `chrono` - Date formatting

---

## Testing Guidelines

- Add unit tests in the same file under `#[cfg(test)]` module
- Use integration tests in `tests/` directory
- Mock git operations when possible
- Test error cases, not just happy paths

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        // Act
        // Assert
    }
}
```

---

## Pre-commit Checklist

Before committing changes:
1. `cargo build` succeeds
2. `cargo test` passes
3. `cargo fmt` applied
4. `cargo clippy` shows no warnings
5. Code follows naming conventions above
