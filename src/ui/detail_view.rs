use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::git::graph::GraphRow;
use chrono::{DateTime, Local, TimeZone};

/// Rend le panneau de détail du commit sélectionné.
pub fn render(frame: &mut Frame, graph: &[GraphRow], selected_index: usize, area: Rect) {
    let content: Vec<Line<'static>> = if let Some(row) = graph.get(selected_index) {
        let node = &row.node;
        let datetime: DateTime<Local> = Local
            .timestamp_opt(node.timestamp, 0)
            .single()
            .unwrap_or_else(Local::now);

        let date_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        let oid_str = node.oid.to_string();
        let author = node.author.clone();
        let message = node.message.clone();

        let refs_display = node.refs.join(", ");
        let has_refs = !node.refs.is_empty();

        let parents_str = node
            .parents
            .iter()
            .map(|p| p.to_string()[..7].to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let has_parents = !parents_str.is_empty();

        let mut lines: Vec<Line<'static>> = vec![
            Line::from(vec![
                Span::styled("Commit:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(oid_str, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Auteur:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(author),
            ]),
            Line::from(vec![
                Span::styled("Date:    ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(date_str),
            ]),
        ];

        if has_refs {
            lines.push(Line::from(vec![
                Span::styled("Refs:    ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(refs_display, Style::default().fg(Color::Cyan)),
            ]));
        }

        if has_parents {
            lines.push(Line::from(vec![
                Span::styled("Parents: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(parents_str, Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::raw(message)));

        lines
    } else {
        vec![Line::from("Aucun commit sélectionné")]
    };

    let paragraph =
        Paragraph::new(content).block(Block::default().title(" Détail ").borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}
