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
pub fn render(frame: &mut Frame, current_view: ViewMode, area: Rect) {
    let tabs = vec![
        ("1", "Graph", ViewMode::Graph),
        ("2", "Staging", ViewMode::Staging),
        ("3", "Branches", ViewMode::Branches),
    ];

    let mut spans: Vec<Span> = vec![Span::raw(" ")];

    for (i, (key, label, mode)) in tabs.iter().enumerate() {
        let is_active = *mode == current_view;

        // Style pour l'onglet
        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        // Construction de l'onglet
        spans.push(Span::styled(format!(" {} ", key), style));
        spans.push(Span::styled(format!("{} ", label), style));

        // Séparateur entre les onglets
        if i < tabs.len() - 1 {
            spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        }
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
pub fn render_compact(current_view: ViewMode) -> Line<'static> {
    let tabs = vec![
        ("1:Graph", ViewMode::Graph),
        ("2:Staging", ViewMode::Staging),
        ("3:Branches", ViewMode::Branches),
    ];

    let mut spans: Vec<Span> = Vec::new();

    for (i, (label, mode)) in tabs.iter().enumerate() {
        let is_active = *mode == current_view;

        let style = if is_active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
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
