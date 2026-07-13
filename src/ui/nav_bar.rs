//! Barre de navigation entre les vues principales.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::i18n::{text, text_owned};
use crate::state::ViewMode;
use crate::ui::theme::current_theme;

pub struct NavBarRenderContext {
    pub current_view: ViewMode,
    pub area: Rect,
    pub unresolved_conflicts: usize,
}

/// Rend la barre de navigation avec les onglets.
pub fn render(frame: &mut Frame, ctx: NavBarRenderContext) {
    let NavBarRenderContext {
        current_view,
        area,
        unresolved_conflicts,
    } = ctx;

    let theme = current_theme();
    let has_conflicts = unresolved_conflicts > 0;

    // Label pour l'onglet Conflits (si applicable)
    let conflicts_label = if has_conflicts {
        Some(if unresolved_conflicts == 1 {
            text_owned("Conflits (1)", "Conflicts (1)")
        } else {
            text_owned(
                format!("Conflits ({})", unresolved_conflicts),
                format!("Conflicts ({})", unresolved_conflicts),
            )
        })
    } else {
        None
    };

    let mut spans: Vec<Span> = vec![Span::raw(" ")];

    // Onglets fixes
    let tabs = [
        ("1", text("Graphe", "Graph"), ViewMode::Graph),
        ("2", text("Staging", "Staging"), ViewMode::Staging),
        ("3", text("Branches", "Branches"), ViewMode::Branches),
        ("4", text("Arbre", "Tree"), ViewMode::ProjectTree),
    ];

    // Rendre les onglets fixes
    for (key, label, mode) in tabs.iter() {
        let is_active = *mode == current_view;

        let style = if is_active {
            Style::default()
                .fg(theme.selection_fg)
                .bg(theme.selection_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_secondary)
        };

        spans.push(Span::styled(format!(" {} ", key), style));
        spans.push(Span::styled(format!("{} ", label), style));
        spans.push(Span::styled(
            "│",
            Style::default().fg(theme.border_inactive),
        ));
    }

    // Onglet Conflits (si applicable)
    if let Some(ref label) = conflicts_label {
        let is_active = ViewMode::Conflicts == current_view;

        let style = if is_active {
            Style::default()
                .fg(theme.selection_fg)
                .bg(theme.error)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.error)
        };

        spans.push(Span::styled(" 5 ", style));
        spans.push(Span::styled(format!("{} ", label), style));
    } else {
        // Retirer le dernier séparateur si pas d'onglet Conflits
        spans.pop();
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line)
        .style(Style::default().bg(theme.surface))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border_inactive))
                .style(Style::default().bg(theme.surface)),
        );

    frame.render_widget(paragraph, area);
}
