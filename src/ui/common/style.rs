//! Styles communs pour l'interface.

use ratatui::style::{Color, Modifier, Style};

use crate::ui::theme::current_theme;

/// Couleur de bordure quand un panel a le focus.
pub const FOCUS_COLOR: Color = Color::Cyan;

/// Couleur de bordure inactive.
pub const INACTIVE_COLOR: Color = Color::Gray;

/// Retourne le style de bordure selon l'état de focus.
pub fn border_style(is_focused: bool) -> Style {
    let theme = current_theme();
    if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    }
}

/// Style pour les éléments sélectionnés dans une liste.
pub fn highlight_style() -> Style {
    let theme = current_theme();
    Style::default()
        .bg(theme.selection_bg)
        .fg(theme.selection_fg)
        .add_modifier(Modifier::BOLD)
}

/// Style pour les titres de section.
pub fn title_style() -> Style {
    let theme = current_theme();
    Style::default()
        .fg(theme.text_normal)
        .add_modifier(Modifier::BOLD)
}

/// Style pour le texte désactivé/secondaire.
pub fn dim_style() -> Style {
    let theme = current_theme();
    Style::default()
        .fg(theme.text_secondary)
        .add_modifier(Modifier::DIM)
}

/// Style pour les messages d'erreur.
pub fn error_style() -> Style {
    let theme = current_theme();
    Style::default().fg(theme.error)
}

/// Style pour les messages de succès.
pub fn success_style() -> Style {
    let theme = current_theme();
    Style::default().fg(theme.success)
}

/// Style pour les ajouts dans les diffs.
pub fn diff_add_style() -> Style {
    let theme = current_theme();
    Style::default().fg(theme.success)
}

/// Style pour les suppressions dans les diffs.
pub fn diff_remove_style() -> Style {
    let theme = current_theme();
    Style::default().fg(theme.error)
}

/// Style pour les headers dans les diffs.
pub fn diff_header_style() -> Style {
    let theme = current_theme();
    Style::default().fg(theme.info)
}
