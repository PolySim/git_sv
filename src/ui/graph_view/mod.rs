//! Rendu du graphe git (colonnes, connexions, couleurs, sélection).

mod header;
mod lines;

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::git::graph::GraphRow;
use crate::ui::theme::current_theme;

use self::header::{build_empty_state_line, build_title};
use self::lines::{build_commit_line, build_connection_line};

/// Espacement entre les colonnes (en caractères).
const COL_SPACING: usize = 2;

/// Contexte de rendu du panneau de graphe.
pub struct GraphRenderContext<'a> {
    pub graph: &'a [GraphRow],
    pub current_branch: Option<&'a str>,
    pub filter_active: bool,
    pub selected_index: usize,
    pub loaded_count: usize,
    pub total_commits: Option<usize>,
    pub can_load_more: bool,
    pub is_loading_more: bool,
    pub area: Rect,
    pub state: &'a mut ListState,
    pub is_focused: bool,
}

/// Rend le graphe de commits dans la zone donnée.
pub fn render(frame: &mut Frame, ctx: GraphRenderContext<'_>) {
    let GraphRenderContext {
        graph,
        current_branch,
        filter_active,
        selected_index,
        loaded_count,
        total_commits,
        can_load_more,
        is_loading_more,
        area,
        state,
        is_focused,
    } = ctx;

    let theme = current_theme();
    let content_width = area.width.saturating_sub(2);
    let visible_height = area.height.saturating_sub(2) as usize;
    let visible_commits = (visible_height / 2).max(1);
    let scroll_offset = selected_index.saturating_sub(visible_commits / 2);

    let (items, selected_visual_index) = if graph.is_empty() {
        (
            vec![ListItem::new(build_empty_state_line(filter_active))],
            Some(0),
        )
    } else {
        build_graph_items(
            graph,
            selected_index,
            content_width,
            scroll_offset,
            visible_commits,
        )
    };

    state.select(selected_visual_index);

    let branch_name = current_branch.unwrap_or("???");
    let title = build_title(
        branch_name,
        selected_index,
        graph.len(),
        loaded_count,
        total_commits,
        can_load_more,
        is_loading_more,
    );

    let title = if is_focused {
        format!("▶{}", title)
    } else {
        title
    };

    let border_style = if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default());

    frame.render_stateful_widget(list, area, state);
}

fn build_graph_items(
    graph: &[GraphRow],
    selected_index: usize,
    available_width: u16,
    scroll_offset: usize,
    visible_count: usize,
) -> (Vec<ListItem<'static>>, Option<usize>) {
    let mut items = Vec::with_capacity(visible_count.saturating_mul(2));
    let mut selected_visual_index = None;

    let end_offset = (scroll_offset + visible_count).min(graph.len());
    let visible_rows = &graph[scroll_offset..end_offset];
    let max_graph_cols = visible_rows
        .iter()
        .map(|row| row.cells.len().max(row.node.column + 1))
        .max()
        .unwrap_or(1);

    for (relative_index, row) in visible_rows.iter().enumerate() {
        let absolute_index = scroll_offset + relative_index;
        let is_selected = absolute_index == selected_index;

        if is_selected {
            selected_visual_index = Some(items.len());
        }

        items.push(ListItem::new(build_commit_line(
            row,
            is_selected,
            available_width,
            max_graph_cols,
        )));

        if let Some(ref connection) = row.connection {
            items.push(ListItem::new(build_connection_line(connection)));
        }
    }

    (items, selected_visual_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{
        CommitNode, ConnectionRow, EdgeType, GraphCell, GraphRow, RefInfo, RefType,
    };
    use crate::i18n::{with_language, Language};
    use crate::ui::graph_view::lines::{find_horizontal_color_bounded, get_branch_color};
    use git2::Oid;
    use ratatui::widgets::ListState;

    fn create_test_graph() -> Vec<GraphRow> {
        vec![
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero()),
                    message: "First commit".to_string(),
                    author: "Alice".to_string(),
                    timestamp: 1609459200,
                    parents: vec![],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell {
                    edge_type: EdgeType::Vertical,
                    color_index: 0,
                })],
                connection: None,
            },
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[2; 20]).unwrap_or(Oid::zero()),
                    message: "Second commit".to_string(),
                    author: "Bob".to_string(),
                    timestamp: 1609545600,
                    parents: vec![Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero())],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell {
                    edge_type: EdgeType::Vertical,
                    color_index: 0,
                })],
                connection: None,
            },
        ]
    }

    #[test]
    fn test_build_graph_items() {
        let graph = create_test_graph();
        let (items, selected_visual_index) = build_graph_items(&graph, 0, 80, 0, graph.len());

        assert!(!items.is_empty());
        assert!(items.len() >= graph.len());
        assert_eq!(selected_visual_index, Some(0));
    }

    #[test]
    fn test_build_graph_items_limits_to_visible_window() {
        let graph = create_test_graph();
        let (items, selected_visual_index) = build_graph_items(&graph, 1, 80, 1, 1);

        assert_eq!(items.len(), 1);
        assert_eq!(selected_visual_index, Some(0));
    }

    #[test]
    fn test_build_commit_line() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, false, 80, 2);
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(line_text.contains("First commit"));
    }

    #[test]
    fn test_build_commit_line_selected() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, true, 80, 2);
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_head_commit_has_distinct_landmarks() {
        let mut row = create_test_graph()[0].clone();
        row.node.refs = vec![
            RefInfo::new("main", RefType::LocalBranch),
            RefInfo::new("main", RefType::Head),
        ];

        let line = build_commit_line(&row, true, 120, 2);
        let line_text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        let head_badge = line
            .spans
            .iter()
            .find(|span| span.content.contains("HEAD:main"))
            .expect("Le badge HEAD doit etre visible");

        assert!(line_text.contains('◆'));
        assert!(line_text.contains("▶"));
        assert_eq!(head_badge.style.bg, Some(current_theme().success));
    }

    #[test]
    fn test_selected_commit_fills_available_width() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, true, 80, 2);

        assert_eq!(line.width(), 80);
        assert!(line.spans.iter().all(|span| span.style.bg.is_some()));
    }

    #[test]
    fn test_many_refs_are_collapsed_into_summary() {
        let mut row = create_test_graph()[0].clone();
        row.node.refs = (0..8)
            .map(|index| RefInfo::new(format!("feature/long-name-{index}"), RefType::LocalBranch))
            .collect();

        let line = build_commit_line(&row, false, 100, 2);
        let line_text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert!(line_text.contains(" +"));
    }

    #[test]
    fn test_unicode_message_truncation_stays_on_character_boundaries() {
        let mut row = create_test_graph()[0].clone();
        row.node.message = "évolution ".repeat(30);

        let line = build_commit_line(&row, false, 70, 2);
        let line_text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();

        assert!(line_text.contains('…'));
        assert!(line_text.contains("évolution"));
    }

    #[test]
    fn test_graph_title_shows_selection_position() {
        let title = build_title("main", 41, 177, 177, Some(177), false, false);

        assert!(title.contains("42/177"));
        assert!(title.contains("main"));
    }

    #[test]
    fn test_build_commit_line_renders_branch_closure_symbol() {
        let row = GraphRow {
            node: CommitNode {
                oid: Oid::from_bytes(&[3; 20]).unwrap_or(Oid::zero()),
                message: "Branch closes".to_string(),
                author: "Alice".to_string(),
                timestamp: 1609459200,
                parents: vec![Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero())],
                refs: vec![],
                branch_name: None,
                column: 0,
                color_index: 0,
            },
            cells: vec![
                None,
                Some(GraphCell {
                    edge_type: EdgeType::MergeFromLeft,
                    color_index: 1,
                }),
            ],
            connection: None,
        };

        let line = build_commit_line(&row, false, 80, 2);
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(line_text.contains("╯"));
    }

    #[test]
    fn test_graph_view_render_basic() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let graph = create_test_graph();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = ListState::default();
        state.select(Some(0));

        terminal
            .draw(|frame| {
                let area = frame.area();
                render(
                    frame,
                    GraphRenderContext {
                        graph: &graph,
                        current_branch: Some("main"),
                        filter_active: false,
                        selected_index: 0,
                        loaded_count: graph.len(),
                        total_commits: Some(graph.len()),
                        can_load_more: false,
                        is_loading_more: false,
                        area,
                        state: &mut state,
                        is_focused: true,
                    },
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty());
    }

    #[test]
    fn test_graph_view_with_selection() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let graph = create_test_graph();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = ListState::default();
        state.select(Some(2));

        terminal
            .draw(|frame| {
                let area = frame.area();
                render(
                    frame,
                    GraphRenderContext {
                        graph: &graph,
                        current_branch: Some("feature"),
                        filter_active: false,
                        selected_index: 1,
                        loaded_count: graph.len(),
                        total_commits: Some(graph.len()),
                        can_load_more: false,
                        is_loading_more: false,
                        area,
                        state: &mut state,
                        is_focused: false,
                    },
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty());
    }

    #[test]
    fn test_get_branch_color() {
        let color0 = get_branch_color(0);
        let color1 = get_branch_color(1);
        let color2 = get_branch_color(2);

        assert_ne!(color0, color1);
        assert_ne!(color1, color2);
    }

    #[test]
    fn test_selected_commit_line_all_spans_have_bg() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, true, 80, 2);
        let theme = current_theme();

        let spans_with_selection_bg: Vec<_> = line
            .spans
            .iter()
            .filter(|s| s.style.bg == Some(theme.selection_bg))
            .collect();

        assert!(spans_with_selection_bg.len() >= 3);

        let message_span = line
            .spans
            .iter()
            .find(|s| s.content.contains("First commit"))
            .expect("Devrait trouver le span du message");
        assert_eq!(message_span.style.bg, Some(theme.selection_bg));

        let hash_span = line
            .spans
            .iter()
            .find(|s| s.content.len() == 8 && s.content.trim().len() == 7)
            .expect("Devrait trouver le span du hash (7 caractères + espace)");
        assert_eq!(hash_span.style.bg, Some(theme.selection_bg));
    }

    #[test]
    fn test_unselected_commit_line_no_bg() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, false, 80, 2);
        let spans_with_selection_bg: Vec<_> =
            line.spans.iter().filter(|s| s.style.bg.is_some()).collect();
        assert!(spans_with_selection_bg.is_empty());
    }

    #[test]
    fn test_no_horizontal_leak_past_fork() {
        let connection = ConnectionRow {
            cells: vec![
                Some(GraphCell {
                    edge_type: EdgeType::MergeFromRight,
                    color_index: 0,
                }),
                Some(GraphCell {
                    edge_type: EdgeType::Horizontal,
                    color_index: 0,
                }),
                Some(GraphCell {
                    edge_type: EdgeType::ForkRight,
                    color_index: 0,
                }),
                None,
                Some(GraphCell {
                    edge_type: EdgeType::Vertical,
                    color_index: 1,
                }),
            ],
        };

        let line = build_connection_line(&connection);
        let all_spans: Vec<_> = line.spans.iter().map(|s| s.content.as_ref()).collect();

        assert_eq!(all_spans[6], "  ");

        let after_fork: String = all_spans.iter().skip(6).copied().collect();
        assert!(!after_fork.contains('─'));
    }

    #[test]
    fn test_horizontal_between_merge_and_fork() {
        let connection = ConnectionRow {
            cells: vec![
                Some(GraphCell {
                    edge_type: EdgeType::MergeFromRight,
                    color_index: 0,
                }),
                None,
                Some(GraphCell {
                    edge_type: EdgeType::ForkRight,
                    color_index: 0,
                }),
            ],
        };

        let line = build_connection_line(&connection);
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(line_text.contains("──"));
    }

    #[test]
    fn test_find_horizontal_color_bounded() {
        let connection1 = ConnectionRow {
            cells: vec![
                Some(GraphCell {
                    edge_type: EdgeType::MergeFromRight,
                    color_index: 0,
                }),
                Some(GraphCell {
                    edge_type: EdgeType::Horizontal,
                    color_index: 0,
                }),
                None,
                Some(GraphCell {
                    edge_type: EdgeType::Horizontal,
                    color_index: 0,
                }),
                Some(GraphCell {
                    edge_type: EdgeType::ForkRight,
                    color_index: 0,
                }),
            ],
        };

        assert_eq!(find_horizontal_color_bounded(2, &connection1), Some(0));

        let connection2 = ConnectionRow {
            cells: vec![
                Some(GraphCell {
                    edge_type: EdgeType::MergeFromRight,
                    color_index: 0,
                }),
                None,
                Some(GraphCell {
                    edge_type: EdgeType::ForkRight,
                    color_index: 0,
                }),
            ],
        };

        assert_eq!(find_horizontal_color_bounded(1, &connection2), Some(0));

        let connection3 = ConnectionRow {
            cells: vec![
                Some(GraphCell {
                    edge_type: EdgeType::ForkRight,
                    color_index: 0,
                }),
                None,
                Some(GraphCell {
                    edge_type: EdgeType::Horizontal,
                    color_index: 1,
                }),
            ],
        };

        assert_eq!(find_horizontal_color_bounded(1, &connection3), Some(1));
    }

    #[test]
    fn test_message_truncation() {
        let mut row = create_test_graph()[0].clone();
        row.node.message = "A".repeat(200);

        let line = build_commit_line(&row, false, 120, 2);
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(line_text.contains('…'));
        assert!(!line_text.contains(&"A".repeat(150)));
    }

    #[test]
    fn test_separator_between_graph_and_text() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, false, 80, 2);

        let separator_span = line
            .spans
            .iter()
            .find(|s| s.content == "  ")
            .expect("Devrait trouver le séparateur de 2 espaces");

        assert_eq!(separator_span.content, "  ");
    }

    #[test]
    fn test_author_date_separate_styles() {
        let row = &create_test_graph()[0];
        let line = build_commit_line(row, false, 80, 2);

        let author_idx = line
            .spans
            .iter()
            .position(|s| s.content.contains("Alice"))
            .unwrap();
        let date_span = line
            .spans
            .get(author_idx + 1)
            .expect("Devrait trouver le span de la date après l'auteur");

        assert!(date_span
            .style
            .add_modifier
            .contains(ratatui::style::Modifier::DIM));
    }

    #[test]
    fn test_graph_columns_aligned() {
        let graph = vec![GraphRow {
            node: CommitNode {
                oid: Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero()),
                message: "First".to_string(),
                author: "Alice".to_string(),
                timestamp: 1609459200,
                parents: vec![],
                refs: vec![],
                branch_name: None,
                column: 0,
                color_index: 0,
            },
            cells: vec![Some(GraphCell {
                edge_type: EdgeType::Vertical,
                color_index: 0,
            })],
            connection: None,
        }];

        let line = build_commit_line(&graph[0], false, 80, 3);
        let padding_spans: Vec<_> = line
            .spans
            .iter()
            .filter(|s| s.content.chars().all(|c| c == ' '))
            .collect();

        assert!(!padding_spans.is_empty());
    }

    #[test]
    fn test_empty_graph_shows_filtered_message() {
        with_language(Language::Fr, || {
            let line = build_empty_state_line(true);
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();

            assert!(text.contains("Aucun commit ne correspond aux filtres actifs"));
        });
    }
}
