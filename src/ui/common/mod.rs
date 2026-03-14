//! Widgets et utilitaires UI réutilisables.

use crate::ui::theme::current_theme;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

// Déclaration des sous-modules
pub mod block;
pub mod help_bar;
pub mod list;
pub mod popup;
pub mod rect;
pub mod style;
pub mod text;

pub use rect::centered_rect;

/// Configuration pour une status bar.
#[derive(Default)]
pub struct StatusBarConfig<'a> {
    /// Titre de la vue (ex: "graph", "staging", "branches")
    pub view_title: &'a str,
    /// Nom de la branche courante
    pub branch: Option<&'a str>,
    /// Chemin du repository
    pub repo_path: &'a str,
    /// Message flash optionnel
    pub flash_message: Option<&'a str>,
    /// Couleur de fond optionnelle.
    pub bg_color: Option<Color>,
}

/// Rend une status bar standardisée.
///
/// Cette fonction remplace les multiples implémentations de status bar
/// dans staging_view et branches_view.
pub fn render_status_bar(frame: &mut Frame, config: StatusBarConfig<'_>, area: Rect) {
    let theme = current_theme();
    let branch_name = config.branch.unwrap_or("???");
    let bg = config.bg_color.unwrap_or(theme.status_bar_bg);

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
            .fg(theme.status_bar_fg)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    )]);

    frame.render_widget(Paragraph::new(line).style(Style::default().bg(bg)), area);
}
