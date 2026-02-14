use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::git::graph::{Edge, GraphRow};

/// Couleurs assignées aux branches du graphe.
const BRANCH_COLORS: &[Color] = &[
    Color::Green,
    Color::Red,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::LightGreen,
    Color::LightRed,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
];

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

        // Ligne de connexion (edges) si ce n'est pas le dernier commit.
        if i + 1 < graph.len() {
            let edge_line = build_edge_line(row, &graph[i + 1]);
            items.push(ListItem::new(edge_line));
        }
    }

    items
}

/// Construit la ligne d'un commit avec le graphe.
fn build_commit_line(row: &GraphRow, is_selected: bool) -> Line<'static> {
    let node = &row.node;
    let color = get_branch_color(node.color_index);

    // Construire le préfixe graphique.
    let mut spans: Vec<Span<'static>> = Vec::new();

    // Colonnes avant le noeud.
    for col in 0..node.column {
        let edge_char = find_edge_char(col, &row.edges);
        let edge_color = find_edge_color(col, &row.edges);
        spans.push(Span::styled(
            format!("{}", edge_char),
            Style::default().fg(edge_color),
        ));
        spans.push(Span::raw(" "));
    }

    // Le noeud lui-même.
    spans.push(Span::styled("●", Style::default().fg(color)));
    spans.push(Span::raw(" "));

    // Colonnes après le noeud.
    let max_col = row
        .edges
        .iter()
        .map(|e| e.from_col.max(e.to_col))
        .max()
        .unwrap_or(node.column);

    for col in node.column + 1..=max_col {
        let edge_char = find_edge_char(col, &row.edges);
        let edge_color = find_edge_color(col, &row.edges);
        spans.push(Span::styled(
            format!("{}", edge_char),
            Style::default().fg(edge_color),
        ));
        spans.push(Span::raw(" "));
    }

    // Hash du commit.
    let hash = node.oid.to_string();
    let short_hash = &hash[..7];
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

    // Auteur.
    spans.push(Span::styled(
        format!(" — {}", node.author),
        Style::default().fg(Color::DarkGray),
    ));

    Line::from(spans)
}

/// Construit la ligne de connexion entre deux commits.
fn build_edge_line(current_row: &GraphRow, next_row: &GraphRow) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    // Trouver le nombre maximum de colonnes à afficher.
    let max_col = current_row
        .edges
        .iter()
        .map(|e| e.from_col.max(e.to_col))
        .max()
        .unwrap_or(0)
        .max(
            next_row
                .edges
                .iter()
                .map(|e| e.from_col.max(e.to_col))
                .max()
                .unwrap_or(0),
        )
        .max(next_row.node.column);

    for col in 0..=max_col {
        // Chercher un edge sur cette colonne.
        let edge = current_row.edges.iter().find(|e| e.from_col == col);

        if let Some(edge) = edge {
            let color = get_branch_color(edge.color_index);

            if edge.from_col == edge.to_col {
                // Ligne verticale.
                spans.push(Span::styled("│", Style::default().fg(color)));
            } else if edge.to_col > edge.from_col {
                // Fork vers la droite.
                spans.push(Span::styled("╭", Style::default().fg(color)));
                spans.push(Span::styled("─", Style::default().fg(color)));
            } else {
                // Merge depuis la droite.
                spans.push(Span::styled("╰", Style::default().fg(color)));
                spans.push(Span::styled("─", Style::default().fg(color)));
            }
        } else {
            // Chercher si un edge traverse cette colonne (diagonale).
            let crossing = current_row.edges.iter().any(|e| {
                let min_col = e.from_col.min(e.to_col);
                let max_col = e.from_col.max(e.to_col);
                col > min_col && col < max_col
            });

            if crossing {
                // Ligne horizontale pour la connexion.
                spans.push(Span::styled("─", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled("─", Style::default().fg(Color::DarkGray)));
            } else {
                spans.push(Span::raw("  "));
            }
        }
    }

    Line::from(spans)
}

/// Retourne la couleur pour un index de branche.
fn get_branch_color(index: usize) -> Color {
    BRANCH_COLORS[index % BRANCH_COLORS.len()]
}

/// Trouve le caractère à afficher pour une colonne donnée.
fn find_edge_char(col: usize, edges: &[Edge]) -> char {
    for edge in edges {
        if edge.from_col == col || edge.to_col == col {
            if edge.from_col == edge.to_col {
                return '│';
            } else if edge.to_col > edge.from_col {
                return '╭';
            } else {
                return '╰';
            }
        }
    }
    ' '
}

/// Trouve la couleur pour une colonne donnée.
fn find_edge_color(col: usize, edges: &[Edge]) -> Color {
    for edge in edges {
        if edge.from_col == col || edge.to_col == col {
            return get_branch_color(edge.color_index);
        }
    }
    Color::Reset
}
