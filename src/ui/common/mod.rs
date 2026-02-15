//! Widgets et utilitaires UI réutilisables.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Calcule un rectangle centré de dimensions données (en pourcentage).
///
/// # Arguments
/// * `percent_x` - Pourcentage de largeur (0-100)
/// * `percent_y` - Pourcentage de hauteur (0-100)
/// * `r` - Rectangle de référence dans lequel centrer
///
/// # Exemple
/// ```rust,ignore
/// let popup_area = centered_rect(70, 80, frame.area());
/// ```
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Configuration pour une status bar.
pub struct StatusBarConfig<'a> {
    /// Titre de la vue (ex: "graph", "staging", "branches")
    pub view_title: &'a str,
    /// Nom de la branche courante
    pub branch: Option<&'a str>,
    /// Chemin du repository
    pub repo_path: &'a str,
    /// Message flash optionnel
    pub flash_message: Option<&'a str>,
    /// Couleur de fond (défaut: Cyan)
    pub bg_color: Option<Color>,
}

impl<'a> Default for StatusBarConfig<'a> {
    fn default() -> Self {
        Self {
            view_title: "",
            branch: None,
            repo_path: "",
            flash_message: None,
            bg_color: Some(Color::Cyan),
        }
    }
}

/// Rend une status bar standardisée.
///
/// Cette fonction remplace les multiples implémentations de status bar
/// dans staging_view et branches_view.
pub fn render_status_bar(frame: &mut Frame, config: StatusBarConfig<'_>, area: Rect) {
    let branch_name = config.branch.unwrap_or("???");
    let bg = config.bg_color.unwrap_or(Color::Cyan);

    let content = if let Some(msg) = config.flash_message {
        format!(
            " git_sv · {} · {} · {} · {} ",
            config.view_title, config.repo_path, branch_name, msg
        )
    } else {
        format!(
            " git_sv · {} · {} · {} ",
            config.view_title, config.repo_path, branch_name
        )
    };

    let line = Line::from(vec![Span::styled(
        content,
        Style::default()
            .fg(Color::Black)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    )]);

    frame.render_widget(Paragraph::new(line).style(Style::default().bg(bg)), area);
}
