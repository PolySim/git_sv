//! Utilitaires divers pour l'application.

pub mod flash;
pub mod terminal;
pub mod time;

pub use flash::{flash_error, flash_error_message, flash_success};
pub use terminal::{terminal_path, terminal_text};
pub use time::{format_absolute_time, format_relative_time};
