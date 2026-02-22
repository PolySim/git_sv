use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::git::graph::{EdgeType, GraphRow};
use crate::ui::theme;
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
    // Construire les lignes du graphe avec les edges de connexion.
    let items = build_graph_items(graph, selected_index);

    let branch_name = current_branch.as_deref().unwrap_or("???");
    let title = format!(" Graphe — {} ", branch_name);

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
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
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, state);
}

/// Construit les items de la liste avec le graphe enrichi.
fn build_graph_items(graph: &[GraphRow], selected_index: usize) -> Vec<ListItem> {
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
        Style::default().fg(Color::Yellow),
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
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    spans.push(Span::styled(node.message.clone(), message_style));

    // Auteur et date relative.
    let relative_date = format_relative_time(node.timestamp);
    spans.push(Span::styled(
        format!(" — {} ({})", node.author, relative_date),
        Style::default().fg(Color::DarkGray),
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
    theme::branch_color(index)
}
