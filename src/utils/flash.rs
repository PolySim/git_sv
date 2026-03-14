//! Helpers pour standardiser les messages flash.

use std::fmt::Display;

/// Formate un message flash de succès.
pub fn flash_success(message: impl Display) -> String {
    format!("{} ✓", message)
}

/// Formate un message flash d'erreur avec le nom de l'opération.
pub fn flash_error(operation: &str, error: impl Display) -> String {
    format!("Erreur {}: {}", operation, error)
}

/// Formate un message flash d'erreur libre.
pub fn flash_error_message(message: impl Display) -> String {
    format!("Erreur: {}", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_success() {
        assert_eq!(flash_success("Opération réussie"), "Opération réussie ✓");
    }

    #[test]
    fn test_flash_error() {
        assert_eq!(flash_error("checkout", "boom"), "Erreur checkout: boom");
    }

    #[test]
    fn test_flash_error_message() {
        assert_eq!(flash_error_message("boom"), "Erreur: boom");
    }
}
