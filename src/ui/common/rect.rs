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

/// Crée un rectangle centré de taille fixe, borné par la zone disponible.
pub fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_rect_fixed_is_centered_and_bounded() {
        assert_eq!(
            centered_rect_fixed(60, 12, Rect::new(0, 0, 80, 24)),
            Rect::new(10, 6, 60, 12)
        );
        assert_eq!(
            centered_rect_fixed(60, 12, Rect::new(0, 0, 40, 8)),
            Rect::new(0, 0, 40, 8)
        );
    }
}
