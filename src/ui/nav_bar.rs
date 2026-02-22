//! Barre de navigation entre les vues principales.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::ViewMode;

/// Rend la barre de navigation avec les onglets.
pub fn render(frame: &mut Frame, current_view: ViewMode, area: Rect, unresolved_conflicts: usize) {
    let has_conflicts = unresolved_conflicts > 0;

    // Label pour l'onglet Conflits (si applicable)
    let conflicts_label = if has_conflicts {
        Some(if unresolved_conflicts == 1 {
            "Conflits (1)".to_string()
        } else {
            format!("Conflits ({})", unresolved_conflicts)
        })
    } else {
        None
    };

    let mut spans: Vec<Span> = vec![Span::raw(" ")];

    // Onglets fixes
    let tabs = vec![
        ("1", "Graph", ViewMode::Graph),
        ("2", "Staging", ViewMode::Staging),
        ("3", "Branches", ViewMode::Branches),
    ];

    // Rendre les onglets fixes
    for (_i, (key, label, mode)) in tabs.iter().enumerate() {
        let is_active = *mode == current_view;

        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        spans.push(Span::styled(format!(" {} ", key), style));
        spans.push(Span::styled(format!("{} ", label), style));
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
    }

    // Onglet Conflits (si applicable)
    if let Some(ref label) = conflicts_label {
        let is_active = ViewMode::Conflicts == current_view;

        let style = if is_active {
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
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
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(paragraph, area);
}

/// Rend une version compacte de la barre de navigation (pour les status bar).
pub fn render_compact(current_view: ViewMode, unresolved_conflicts: usize) -> Line<'static> {
    let has_conflicts = unresolved_conflicts > 0;

    let mut tabs: Vec<(&str, ViewMode)> = vec![
        ("1:Graph", ViewMode::Graph),
        ("2:Staging", ViewMode::Staging),
        ("3:Branches", ViewMode::Branches),
    ];

    // Ajouter l'onglet Conflits s'il y a des conflits
    // Note: on n'affiche pas le nombre en mode compact pour garder la simplicité
    if has_conflicts {
        tabs.push(("4:Conflits", ViewMode::Conflicts));
    }

    let mut spans: Vec<Span> = Vec::new();

    for (i, (label, mode)) in tabs.iter().enumerate() {
        let is_active = *mode == current_view;
        let is_conflicts = *mode == ViewMode::Conflicts;

        let style = if is_active {
            if is_conflicts {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            }
        } else if is_conflicts {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        spans.push(Span::styled(*label, style));

        if i < tabs.len() - 1 {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }
    }

    Line::from(spans)
}
