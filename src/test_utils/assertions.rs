//! Assertions personnalisées pour les tests.

/// Asserts que la sélection est à l'index attendu.
#[macro_export]
macro_rules! assert_selection_eq {
    ($selection:expr, $expected:expr) => {
        assert_eq!(
            $selection.selected_index(),
            $expected,
            "La sélection devrait être à l'index {} mais est à {}",
            $expected,
            $selection.selected_index()
        );
    };
}

/// Asserts que l'état est en mode de vue attendu.
#[macro_export]
macro_rules! assert_view_mode {
    ($state:expr, $expected:expr) => {
        assert_eq!(
            $state.view_mode, $expected,
            "Le mode de vue devrait être {:?} mais est {:?}",
            $expected, $state.view_mode
        );
    };
}

/// Asserts que le message flash contient le texte attendu.
#[macro_export]
macro_rules! assert_flash_contains {
    ($state:expr, $text:expr) => {
        assert!(
            $state
                .flash_message
                .as_ref()
                .map(|(m, _)| m.contains($text))
                .unwrap_or(false),
            "Le message flash devrait contenir '{}' mais est {:?}",
            $text,
            $state.flash_message
        );
    };
}
