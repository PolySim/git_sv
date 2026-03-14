//! Utilitaires divers pour l'application.

pub mod flash;
pub mod time;

pub use flash::{flash_error, flash_error_message, flash_success};
pub use time::{format_absolute_time, format_relative_time};
