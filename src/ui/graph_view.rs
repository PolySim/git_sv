use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::git::graph::{EdgeType, GraphRow};
use crate::ui::theme::{branch_color, current_theme};
use crate::utils::format_relative_time;

/// Espacement entre les colonnes (en caractères).
const COL_SPACING: usize = 2;

/// Rend le graphe de commits dans la zone donnée.
pub fn render(
    frame: &mut Frame,
    graph: &[GraphRow],
    current_branch: &Option<String>,
    selected_index: usize,
    area: Rect,
    state: &mut ListState,
    is_focused: bool,
) {
    let theme = current_theme();
    
    // Construire les lignes du graphe avec les edges de connexion.
    let items = build_graph_items(graph, selected_index);

    let branch_name = current_branch.as_deref().unwrap_or("???");
    let title = format!(" Graphe — {} ", branch_name);

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
        .highlight_style(
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, state);
}

/// Construit les items de la liste avec le graphe enrichi.
fn build_graph_items(graph: &[GraphRow], selected_index: usize) -> Vec<ListItem<'static>> {
    let mut items = Vec::with_capacity(graph.len() * 2);

    for (i, row) in graph.iter().enumerate() {
        let is_selected = i == selected_index;

        // Ligne du commit.
        let commit_line = build_commit_line(row, is_selected);
        items.push(ListItem::new(commit_line));

        // Ligne de connexion vers le commit suivant (si existe).
        if let Some(ref connection) = row.connection {
            let connection_line = build_connection_line(connection);
            items.push(ListItem::new(connection_line));
        }
    }

    items
}

/// Construit la ligne d'un commit avec le graphe.
fn build_commit_line(row: &GraphRow, is_selected: bool) -> Line<'static> {
    let theme = current_theme();
    let node = &row.node;
    let commit_color = get_branch_color(node.color_index);

    let mut spans: Vec<Span<'static>> = Vec::new();

    // Nombre total de colonnes à afficher.
    let num_cols = row.cells.len().max(node.column + 1);

    // Construire chaque colonne avec l'espacement approprié.
    for col in 0..num_cols {
        if col == node.column {
            // C'est la colonne du commit - dessiner le nœud.
            let symbol = if node.parents.len() > 1 { "○" } else { "●" };
            spans.push(Span::styled(
                symbol.to_string(),
                Style::default()
                    .fg(commit_color)
                    .add_modifier(Modifier::BOLD),
            ));

            // Ajouter l'espacement après le nœud (sauf si c'est la dernière colonne).
            if col < num_cols - 1 {
                spans.push(Span::raw(" ".repeat(COL_SPACING - 1)));
            }
        } else if col < row.cells.len() {
            // Colonne avec potentiellement une cellule graphique.
            if let Some(ref cell) = row.cells[col] {
                let color = get_branch_color(cell.color_index);
                let ch = match cell.edge_type {
                    EdgeType::Vertical => "│",
                    _ => " ",
                };
                spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));

                // Ajouter l'espacement.
                if col < num_cols - 1 {
                    spans.push(Span::raw(" ".repeat(COL_SPACING - 1)));
                }
            } else {
                // Colonne vide.
                let spaces = if col < num_cols - 1 {
                    " ".repeat(COL_SPACING)
                } else {
                    " ".to_string()
                };
                spans.push(Span::raw(spaces));
            }
        } else {
            // Colonne au-delà des cellules définies - juste des espaces.
            let spaces = if col < num_cols - 1 {
                " ".repeat(COL_SPACING)
            } else {
                " ".to_string()
            };
            spans.push(Span::raw(spaces));
        }
    }

    // Espace entre le graphe et le contenu.
    spans.push(Span::raw(" "));

    // Hash du commit.
    let hash = node.oid.to_string();
    let short_hash = if hash.len() >= 7 { &hash[..7] } else { &hash };
    spans.push(Span::styled(
        format!("{} ", short_hash),
        Style::default().fg(theme.commit_hash),
    ));

    // Labels de branches si présents.
    if !node.refs.is_empty() {
        for ref_name in &node.refs {
            let ref_color = get_branch_color(node.color_index);
            spans.push(Span::styled(
                format!("[{}] ", ref_name),
                Style::default()
                    .fg(ref_color)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::REVERSED),
            ));
        }
    }

    // Message du commit.
    let message_style = if is_selected {
        Style::default()
            .bg(theme.selection_bg)
            .fg(theme.selection_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_normal)
    };
    spans.push(Span::styled(node.message.clone(), message_style));

    // Auteur et date relative.
    let relative_date = format_relative_time(node.timestamp);
    spans.push(Span::styled(
        format!(" — {} ({})", node.author, relative_date),
        Style::default().fg(theme.text_secondary),
    ));

    Line::from(spans)
}

/// Construit la ligne de connexion entre deux commits.
fn build_connection_line(connection: &crate::git::graph::ConnectionRow) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    let num_cols = connection.cells.len();

    for col in 0..num_cols {
        if let Some(ref cell) = connection.cells[col] {
            let color = get_branch_color(cell.color_index);
            let ch = match cell.edge_type {
                EdgeType::Vertical => "│",
                EdgeType::ForkRight => "╮",
                EdgeType::ForkLeft => "╭",
                EdgeType::MergeFromRight => "╰",
                EdgeType::MergeFromLeft => "╯",
                EdgeType::Horizontal => "─",
                EdgeType::Cross => "┼",
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));

            // Ajouter l'espacement entre les colonnes.
            if col < num_cols - 1 {
                // Vérifier s'il y a une ligne horizontale à continuer.
                // Une ligne horizontale continue seulement si la cellule suivante
                // est Horizontal (pas ForkRight/ForkLeft qui sont des points d'arrivée).
                let needs_horizontal_right = col + 1 < num_cols
                    && connection.cells[col + 1]
                        .as_ref()
                        .map_or(false, |c| c.edge_type == EdgeType::Horizontal);
                let needs_horizontal_left = col > 0
                    && connection.cells[col - 1]
                        .as_ref()
                        .map_or(false, |c| c.edge_type == EdgeType::Horizontal);

                if needs_horizontal_right || needs_horizontal_left {
                    spans.push(Span::styled("─", Style::default().fg(color)));
                } else {
                    spans.push(Span::raw(" "));
                }
            }
        } else {
            // Colonne vide - vérifier s'il y a une ligne horizontale qui traverse.
            let has_horizontal = connection.cells.iter().any(|c| {
                c.as_ref()
                    .map_or(false, |cell| cell.edge_type == EdgeType::Horizontal)
            });

            if has_horizontal {
                // Chercher la couleur d'une ligne horizontale adjacente.
                let horizontal_color = find_horizontal_color(col, connection);
                if let Some(color_idx) = horizontal_color {
                    let color = get_branch_color(color_idx);
                    spans.push(Span::styled("─", Style::default().fg(color)));
                    spans.push(Span::styled("─", Style::default().fg(color)));
                } else {
                    spans.push(Span::raw("  "));
                }
            } else {
                spans.push(Span::raw("  "));
            }
        }
    }

    Line::from(spans)
}

/// Trouve la couleur d'une ligne horizontale adjacente.
fn find_horizontal_color(
    col: usize,
    connection: &crate::git::graph::ConnectionRow,
) -> Option<usize> {
    // Chercher à gauche.
    for c in (0..=col).rev() {
        if let Some(ref cell) = connection.cells[c] {
            if cell.edge_type == EdgeType::Horizontal {
                return Some(cell.color_index);
            }
        }
    }

    // Chercher à droite.
    for c in col..connection.cells.len() {
        if let Some(ref cell) = connection.cells[c] {
            if cell.edge_type == EdgeType::Horizontal {
                return Some(cell.color_index);
            }
        }
    }

    None
}

/// Retourne la couleur pour un index de branche.
fn get_branch_color(index: usize) -> Color {
    branch_color(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{GraphRow, CommitNode, GraphCell, EdgeType};
    use git2::Oid;
    use ratatui::widgets::ListState;

    fn create_test_graph() -> Vec<GraphRow> {
        vec![
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero()),
                    message: "First commit".to_string(),
                    author: "Alice".to_string(),
                    timestamp: 1609459200, // 2021-01-01
                    parents: vec![],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell { edge_type: EdgeType::Vertical, color_index: 0 })],
                connection: None,
            },
            GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[2; 20]).unwrap_or(Oid::zero()),
                    message: "Second commit".to_string(),
                    author: "Bob".to_string(),
                    timestamp: 1609545600, // 2021-01-02
                    parents: vec![Oid::from_bytes(&[1; 20]).unwrap_or(Oid::zero())],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![Some(GraphCell { edge_type: EdgeType::Vertical, color_index: 0 })],
                connection: None,
            },
        ]
    }

    #[test]
    fn test_build_graph_items() {
        let graph = create_test_graph();
        let theme = current_theme();
        let items = build_graph_items(&graph, 0, theme);

        // Chaque GraphRow génère au moins 1 item
        assert!(!items.is_empty());
        assert!(items.len() >= graph.len());
    }

    #[test]
    fn test_build_commit_line() {
        let row = &create_test_graph()[0];
        let theme = current_theme();
        let line = build_commit_line(row, false, theme);

        // La ligne devrait contenir le message
        let line_text: String = line.spans.iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(line_text.contains("First commit"));
    }

    #[test]
    fn test_build_commit_line_selected() {
        let row = &create_test_graph()[0];
        let theme = current_theme();
        let line = build_commit_line(row, true, theme);

        // La ligne devrait avoir des spans
        assert!(!line.spans.is_empty());
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

        terminal.draw(|frame| {
            let area = frame.area();
            render(
                frame,
                &graph,
                &Some("main".to_string()),
                0,
                area,
                &mut state,
                true,
            );
        }).unwrap();

        // Vérifier que quelque chose a été rendu
        let buffer = terminal.backend().buffer();
        assert!(buffer.content.len() > 0);
    }

    #[test]
    fn test_graph_view_with_selection() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let graph = create_test_graph();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = ListState::default();
        state.select(Some(2)); // Sélectionner le deuxième commit

        terminal.draw(|frame| {
            let area = frame.area();
            render(
                frame,
                &graph,
                &Some("feature".to_string()),
                1, // selected_index = 1
                area,
                &mut state,
                false,
            );
        }).unwrap();

        let buffer = terminal.backend().buffer();
        assert!(buffer.content.len() > 0);
    }

    #[test]
    fn test_get_branch_color() {
        let color0 = get_branch_color(0);
        let color1 = get_branch_color(1);
        let color2 = get_branch_color(2);

        // Les couleurs devraient être différentes
        assert_ne!(color0, color1);
        assert_ne!(color1, color2);
    }
}
