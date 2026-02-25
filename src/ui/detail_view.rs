use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::git::graph::GraphRow;
use crate::ui::theme::current_theme;
use crate::utils::format_absolute_time;

/// Rend le panneau de détail du commit sélectionné.
pub fn render(
    frame: &mut Frame,
    graph: &[GraphRow],
    selected_index: usize,
    area: Rect,
    is_focused: bool,
) {
    let theme = current_theme();

    let content: Vec<Line<'static>> = if let Some(row) = graph.get(selected_index) {
        let node = &row.node;
        let date_str = format_absolute_time(node.timestamp);
        let oid_str = node.oid.to_string();
        let author = node.author.clone();
        let message = node.message.clone();

        let refs_display: Vec<String> = node.refs.iter().map(|r| r.name.clone()).collect();
        let refs_display = refs_display.join(", ");
        let has_refs = !node.refs.is_empty();

        let parents_str = node
            .parents
            .iter()
            .map(|p| p.to_string()[..7].to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let has_parents = !parents_str.is_empty();

        // Indicateur de type de commit
        let (commit_type, type_style) = if node.parents.len() > 1 {
            (
                "⊕ Merge",
                Style::default()
                    .fg(theme.info)
                    .add_modifier(Modifier::BOLD),
            )
        } else if node.parents.is_empty() {
            (
                "◆ Initial",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                "● Commit",
                Style::default()
                    .fg(theme.text_secondary)
                    .add_modifier(Modifier::BOLD),
            )
        };

        let mut lines: Vec<Line<'static>> = vec![
            // Type de commit
            Line::from(vec![Span::styled(commit_type.to_string(), type_style)]),
            // Commit hash
            Line::from(vec![
                Span::styled(
                    "Commit:  ",
                    Style::default()
                        .fg(theme.text_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(oid_str, Style::default().fg(theme.commit_hash)),
            ]),
            // Auteur
            Line::from(vec![
                Span::styled(
                    "Auteur:  ",
                    Style::default()
                        .fg(theme.text_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(author, Style::default().fg(theme.text_normal)),
            ]),
            // Date
            Line::from(vec![
                Span::styled(
                    "Date:    ",
                    Style::default()
                        .fg(theme.text_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(date_str, Style::default().fg(theme.text_normal)),
            ]),
        ];

        // Refs avec style thème
        if has_refs {
            lines.push(Line::from(vec![
                Span::styled(
                    "Refs:    ",
                    Style::default()
                        .fg(theme.text_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(refs_display, Style::default().fg(theme.primary)),
            ]));
        }

        // Branche
        if let Some(branch) = &node.branch_name {
            lines.push(Line::from(vec![
                Span::styled(
                    "Branche: ",
                    Style::default()
                        .fg(theme.text_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(branch.clone(), Style::default().fg(theme.success)),
            ]));
        }

        // Parents
        if has_parents {
            lines.push(Line::from(vec![
                Span::styled(
                    "Parents: ",
                    Style::default()
                        .fg(theme.text_secondary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(parents_str, Style::default().fg(theme.text_secondary)),
            ]));
        }

        // Séparateur visuel
        lines.push(Line::from(Span::styled(
            "─".repeat(area.width.saturating_sub(2) as usize),
            Style::default().fg(theme.border_inactive),
        )));

        // Message (multi-ligne)
        for msg_line in message.lines() {
            lines.push(Line::from(Span::styled(
                msg_line.to_string(),
                Style::default().fg(theme.text_normal),
            )));
        }

        lines
    } else {
        vec![Line::from(Span::styled(
            "Aucun commit sélectionné",
            Style::default().fg(theme.text_secondary),
        ))]
    };

    let border_style = if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Détail ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{
        CommitNode, ConnectionRow, EdgeType, GraphCell, GraphRow, RefInfo, RefType,
    };
    use crate::ui::theme::current_theme;
    use git2::Oid;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_test_graph() -> Vec<GraphRow> {
        vec![
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero()),
                    message: "First commit\nWith multiple lines".to_string(),
                    author: "Alice".to_string(),
                    timestamp: 1609459200,
                    parents: vec![],
                    refs: vec![
                        RefInfo::new("main", RefType::Head),
                        RefInfo::new("v1.0", RefType::Tag),
                    ],
                    branch_name: Some("main".to_string()),
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell {
                    edge_type: crate::git::graph::EdgeType::Vertical,
                    color_index: 0,
                })],
                connection: None,
            },
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[2; 20]).unwrap_or(Oid::zero()),
                    message: "Merge commit".to_string(),
                    author: "Bob".to_string(),
                    timestamp: 1609545600,
                    parents: vec![
                        Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero()),
                        Oid::from_bytes(&[3; 20]).unwrap_or(Oid::zero()),
                    ],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell {
                    edge_type: crate::git::graph::EdgeType::Vertical,
                    color_index: 0,
                })],
                connection: None,
            },
        ]
    }

    #[test]
    fn test_detail_view_uses_theme() {
        let graph = create_test_graph();
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = current_theme();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render(frame, &graph, 0, area, true);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Vérifier que quelque chose a été rendu
        assert!(buffer.content.len() > 0);

        // Vérifier que le titre est présent
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content.contains("Détail"));
        assert!(content.contains("First commit"));
    }

    #[test]
    fn test_detail_view_merge_indicator() {
        let graph = create_test_graph();
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        // Tester avec le commit de merge (index 1)
        terminal
            .draw(|frame| {
                let area = frame.area();
                render(frame, &graph, 1, area, false);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();

        // Vérifier que l'indicateur de merge est présent
        assert!(content.contains("⊕ Merge") || content.contains("Merge"),
            "Le détail devrait contenir l'indicateur de merge");
    }

    #[test]
    fn test_detail_view_initial_indicator() {
        let graph = create_test_graph();
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        // Tester avec le commit initial (index 0, pas de parents)
        terminal
            .draw(|frame| {
                let area = frame.area();
                render(frame, &graph, 0, area, false);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();

        // Vérifier que l'indicateur initial est présent
        assert!(content.contains("◆ Initial") || content.contains("Initial"),
            "Le détail devrait contenir l'indicateur de commit initial");
    }

    #[test]
    fn test_detail_view_multi_line_message() {
        let graph = create_test_graph();
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render(frame, &graph, 0, area, false);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();

        // Vérifier que les deux lignes du message sont présentes
        assert!(content.contains("First commit"),
            "Le message devrait contenir 'First commit'");
        assert!(content.contains("With multiple lines"),
            "Le message devrait contenir 'With multiple lines'");
    }

    #[test]
    fn test_detail_view_no_selection() {
        let graph: Vec<GraphRow> = vec![];
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render(frame, &graph, 0, area, false);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();

        // Vérifier que le message "Aucun commit sélectionné" est affiché
        assert!(content.contains("Aucun commit"),
            "Devrait afficher 'Aucun commit sélectionné' quand il n'y a pas de graphe");
    }
}
