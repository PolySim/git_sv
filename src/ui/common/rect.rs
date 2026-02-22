//! Utilitaires de calcul de zones rectangulaires.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Crée un rectangle centré dans la zone donnée.
///
/// # Arguments
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

/// Crée un rectangle centré avec dimensions fixes.
pub fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Vérifie si le terminal est suffisamment grand.
pub fn is_terminal_size_adequate(area: Rect, min_width: u16, min_height: u16) -> bool {
    area.width >= min_width && area.height >= min_height
}
