//! Styles communs pour l'interface.

use ratatui::style::{Color, Modifier, Style};

/// Couleur de bordure quand un panel a le focus.
pub const FOCUS_COLOR: Color = Color::Cyan;

/// Couleur de bordure inactive.
pub const INACTIVE_COLOR: Color = Color::DarkGray;

/// Retourne le style de bordure selon l'état de focus.
pub fn border_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(FOCUS_COLOR)
    } else {
        Style::default().fg(INACTIVE_COLOR)
    }
}

/// Style pour les éléments sélectionnés dans une liste.
pub fn highlight_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

/// Style pour les titres de section.
pub fn title_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Style pour le texte désactivé/secondaire.
pub fn dim_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// Style pour les messages d'erreur.
pub fn error_style() -> Style {
    Style::default().fg(Color::Red)
}

/// Style pour les messages de succès.
pub fn success_style() -> Style {
    Style::default().fg(Color::Green)
}

/// Style pour les ajouts dans les diffs.
pub fn diff_add_style() -> Style {
    Style::default().fg(Color::Green)
}

/// Style pour les suppressions dans les diffs.
pub fn diff_remove_style() -> Style {
    Style::default().fg(Color::Red)
}

/// Style pour les headers dans les diffs.
pub fn diff_header_style() -> Style {
    Style::default().fg(Color::Cyan)
}
