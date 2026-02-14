use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::App;
use crate::git::CommitNode;

/// Couleurs assignées aux colonnes du graphe.
const BRANCH_COLORS: &[Color] = &[
    Color::Green,
    Color::Red,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
];

/// Rend le graphe de commits dans la zone donnée.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .graph
        .iter()
        .enumerate()
        .map(|(i, node)| build_graph_line(node, i == app.selected_index))
        .collect();

    let current_branch = app.current_branch.as_deref().unwrap_or("???");
    let title = format!(" Graphe — {} ", current_branch);

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(list, area);
}

/// Construit une ligne du graphe pour un commit donné.
fn build_graph_line(node: &CommitNode, is_selected: bool) -> ListItem<'static> {
    let color = BRANCH_COLORS[node.column % BRANCH_COLORS.len()];

    // Construire le préfixe graphique (indentation + noeud).
    let mut prefix = String::new();
    for col in 0..node.column {
        let col_color_char = if col < node.column { "│ " } else { "  " };
        prefix.push_str(col_color_char);
    }
    prefix.push_str("● ");

    let hash = node.oid.to_string()[..7].to_string();

    // Construire les refs si présentes.
    let refs_str = if node.refs.is_empty() {
        String::new()
    } else {
        format!(" ({})", node.refs.join(", "))
    };

    let message = node.message.clone();
    let author = node.author.clone();

    let style = if is_selected {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let line = Line::from(vec![
        Span::styled(prefix, Style::default().fg(color)),
        Span::styled(format!("{} ", hash), Style::default().fg(Color::Yellow)),
        Span::styled(message, style),
        Span::styled(
            refs_str,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" — {}", author),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    ListItem::new(line)
}
