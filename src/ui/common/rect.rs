//! Utilitaires de calcul de zones rectangulaires.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Crée un rectangle centré dans la zone donnée.
///
/// # Paramètres
/// * `percent_x` - Pourcentage de largeur (0-100)
/// * `percent_y` - Pourcentage de hauteur (0-100)
/// * `area` - Zone parente
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical_layout[1])[1]
}
