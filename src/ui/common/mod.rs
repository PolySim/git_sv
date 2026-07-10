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

pub use rect::{centered_rect, centered_rect_fixed};
use text::truncate_start;

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

    let path_width = (usize::from(area.width) / 3).clamp(12, 40);
    let repo_path = truncate_start(config.repo_path, path_width, true);
    let mut spans = vec![
        Span::styled(
            " git_sv ",
            Style::default()
                .fg(theme.primary)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("· ", Style::default().fg(theme.text_secondary).bg(bg)),
        Span::styled(
            format!("{} ", config.view_title),
            Style::default()
                .fg(theme.status_bar_fg)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("· ", Style::default().fg(theme.text_secondary).bg(bg)),
        Span::styled(repo_path, Style::default().fg(theme.text_secondary).bg(bg)),
        Span::styled(" · ", Style::default().fg(theme.text_secondary).bg(bg)),
        Span::styled(
            branch_name.to_string(),
            Style::default().fg(theme.commit_hash).bg(bg),
        ),
    ];
    if let Some(message) = config.flash_message {
        spans.push(Span::styled(
            format!("  {message}"),
            Style::default()
                .fg(theme.secondary)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let line = Line::from(spans);

    frame.render_widget(Paragraph::new(line).style(Style::default().bg(bg)), area);
}
