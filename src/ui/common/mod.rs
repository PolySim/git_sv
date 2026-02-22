//! Widgets et utilitaires UI réutilisables.

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

// Re-exports pour un accès plus simple
pub use block::StyledBlock;
pub use help_bar::{HelpBar, KeyBinding};
pub use list::{list_item, list_item_styled, StyledList};
pub use popup::Popup;
pub use rect::{centered_rect, centered_rect_fixed, is_terminal_size_adequate};
pub use style::{
    border_style, diff_add_style, diff_header_style, diff_remove_style, dim_style, error_style,
    highlight_style, success_style, title_style, FOCUS_COLOR, INACTIVE_COLOR,
};
pub use text::{pad_left, pad_right, truncate, truncate_start};

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
