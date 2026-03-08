//! Barre de navigation entre les vues principales.

#![allow(dead_code)]

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

        spans.push(Span::styled(" 4 ", style));
        spans.push(Span::styled(format!("{} ", label), style));
    } else {
        // Retirer le dernier séparateur si pas d'onglet Conflits
        spans.pop();
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.border_inactive)),
    );

    frame.render_widget(paragraph, area);
}

/// Rend une version compacte de la barre de navigation (pour les status bar).
pub fn render_compact(current_view: ViewMode, unresolved_conflicts: usize) -> Line<'static> {
    let theme = current_theme();
    let has_conflicts = unresolved_conflicts > 0;

    let mut tabs: Vec<(&str, ViewMode)> = vec![
        (text("1:Graphe", "1:Graph"), ViewMode::Graph),
        ("2:Staging", ViewMode::Staging),
        ("3:Branches", ViewMode::Branches),
    ];

    // Ajouter l'onglet Conflits s'il y a des conflits
    // Note: on n'affiche pas le nombre en mode compact pour garder la simplicité
    if has_conflicts {
        tabs.push((text("4:Conflits", "4:Conflicts"), ViewMode::Conflicts));
    }

    let mut spans: Vec<Span> = Vec::new();

    for (i, (label, mode)) in tabs.iter().enumerate() {
        let is_active = *mode == current_view;
        let is_conflicts = *mode == ViewMode::Conflicts;

        let style = if is_active {
            if is_conflicts {
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            }
        } else if is_conflicts {
            Style::default().fg(theme.error)
        } else {
            Style::default().fg(theme.text_secondary)
        };

        spans.push(Span::styled(*label, style));

        if i < tabs.len() - 1 {
            spans.push(Span::styled(
                " | ",
                Style::default().fg(theme.border_inactive),
            ));
        }
    }

    Line::from(spans)
}
