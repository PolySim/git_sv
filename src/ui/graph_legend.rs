//! Légende du graphe montrant les couleurs des branches.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::git::graph::{GraphRow, RefType};
use crate::ui::theme;

/// Rend une légende compacte des branches actives dans le graphe.
pub fn render(frame: &mut Frame, graph: &[GraphRow], area: Rect) {
    // Collecter les branches uniques avec leurs couleurs
    let mut branches: Vec<(String, usize)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for row in graph.iter().take(20) {
        // Limiter à 20 commits récents pour la légende
        // Ne prendre que les branches locales et HEAD (pas les tags/remotes)
        for ref_info in &row.node.refs {
            if matches!(ref_info.ref_type, RefType::LocalBranch | RefType::Head)
                && !seen.contains(&ref_info.name)
            {
                seen.insert(ref_info.name.clone());
                branches.push((ref_info.name.clone(), row.node.color_index));
            }
        }
    }

    // Limiter à 5 branches pour la légende
    branches.truncate(5);

    if branches.is_empty() {
        return;
    }

    // Construire la ligne de légende
    let mut spans: Vec<Span> = vec![Span::styled(
        "Branches: ",
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::BOLD),
    )];

    for (i, (name, color_idx)) in branches.iter().enumerate() {
        let color = theme::branch_color(*color_idx);

        spans.push(Span::styled(
            "●",
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {}", name), Style::default()));

        if i < branches.len() - 1 {
            spans.push(Span::raw("  "));
        }
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(paragraph, area);
}

/// Rend une légende minimale (juste les points colorés) pour les espaces réduits.
pub fn render_compact(frame: &mut Frame, graph: &[GraphRow], area: Rect) {
    // Collecter les couleurs uniques des 5 premières branches
    let mut colors: Vec<usize> = Vec::new();
    let mut seen_refs = std::collections::HashSet::new();

    for row in graph.iter().take(20) {
        for ref_info in &row.node.refs {
            if !seen_refs.contains(&ref_info.name) && colors.len() < 5 {
                seen_refs.insert(ref_info.name.clone());
                colors.push(row.node.color_index);
            }
        }
    }

    if colors.is_empty() {
        return;
    }

    let mut spans: Vec<Span> = Vec::new();

    for (i, color_idx) in colors.iter().enumerate() {
        let color = theme::branch_color(*color_idx);
        spans.push(Span::styled(
            "●",
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        if i < colors.len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);

    frame.render_widget(paragraph, area);
}
