use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::git::graph::{ConnectionRow, EdgeType, GraphRow, RefType};
use crate::ui::theme::{branch_color, current_theme};
use crate::utils::format_relative_time;

use super::COL_SPACING;

pub(super) fn build_commit_line(
    row: &GraphRow,
    is_selected: bool,
    available_width: u16,
    max_graph_cols: usize,
) -> Line<'static> {
    let theme = current_theme();
    let node = &row.node;
    let commit_color = get_branch_color(node.color_index);

    let mut spans: Vec<Span<'static>> = Vec::new();
    let num_cols = row.cells.len().max(node.column + 1);

    for col in 0..num_cols {
        if col == node.column {
            let symbol = if node.parents.len() > 1 { "○" } else { "●" };
            spans.push(Span::styled(
                symbol.to_string(),
                Style::default()
                    .fg(commit_color)
                    .add_modifier(Modifier::BOLD),
            ));

            if col < num_cols - 1 {
                spans.push(Span::raw(" ".repeat(COL_SPACING - 1)));
            }
        } else if col < row.cells.len() {
            if let Some(ref cell) = row.cells[col] {
                let color = get_branch_color(cell.color_index);
                let ch = match cell.edge_type {
                    EdgeType::Vertical => "│",
                    EdgeType::Horizontal => "─",
                    EdgeType::Cross => "┼",
                    EdgeType::MergeFromRight => "╰",
                    EdgeType::MergeFromLeft => "╯",
                    EdgeType::ForkRight => "╮",
                    EdgeType::ForkLeft => "╭",
                };
                spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));

                if col < num_cols - 1 {
                    spans.push(Span::raw(" ".repeat(COL_SPACING - 1)));
                }
            } else {
                let spaces = if col < num_cols - 1 {
                    " ".repeat(COL_SPACING)
                } else {
                    " ".to_string()
                };
                spans.push(Span::raw(spaces));
            }
        } else {
            let spaces = if col < num_cols - 1 {
                " ".repeat(COL_SPACING)
            } else {
                " ".to_string()
            };
            spans.push(Span::raw(spaces));
        }
    }

    for _ in num_cols..max_graph_cols {
        spans.push(Span::raw(" ".repeat(COL_SPACING)));
    }

    spans.push(Span::raw("  "));

    let sel_style = |base_fg: Color| -> Style {
        if is_selected {
            Style::default()
                .bg(theme.selection_bg)
                .fg(base_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(base_fg)
        }
    };

    let hash = node.oid.to_string();
    let short_hash = if hash.len() >= 7 { &hash[..7] } else { &hash };
    spans.push(Span::styled(
        format!("{} ", short_hash),
        sel_style(theme.commit_hash),
    ));

    let head_branch_name = node
        .refs
        .iter()
        .find(|r| r.ref_type == RefType::Head)
        .map(|r| r.name.as_str());

    let mut sorted_refs: Vec<_> = node
        .refs
        .iter()
        .filter(|r| {
            !(r.ref_type == RefType::LocalBranch && Some(r.name.as_str()) == head_branch_name)
        })
        .collect();
    sorted_refs.sort_by_key(|r| match r.ref_type {
        RefType::Head => 0,
        RefType::LocalBranch => 1,
        RefType::Tag => 2,
        RefType::RemoteBranch => 3,
    });

    let refs_width: usize = sorted_refs
        .iter()
        .map(|r| {
            let bracket_len = match r.ref_type {
                RefType::Head => 8,
                RefType::Tag => 3,
                RefType::RemoteBranch => 4,
                RefType::LocalBranch => 3,
            };
            r.name.len() + bracket_len
        })
        .sum();

    for ref_info in sorted_refs {
        let (bracket, style) = match ref_info.ref_type {
            RefType::Head => {
                let bracket = format!("HEAD->{} ", ref_info.name);
                let style = sel_style(theme.success).add_modifier(Modifier::BOLD);
                (bracket, style)
            }
            RefType::LocalBranch => {
                let ref_color = get_branch_color(node.color_index);
                let bracket = format!("[{}] ", ref_info.name);
                let style = sel_style(ref_color).add_modifier(Modifier::BOLD | Modifier::REVERSED);
                (bracket, style)
            }
            RefType::RemoteBranch => {
                let bracket = format!("⟨{}⟩ ", ref_info.name);
                let style = sel_style(theme.text_secondary).add_modifier(Modifier::DIM);
                (bracket, style)
            }
            RefType::Tag => {
                let bracket = format!("({}) ", ref_info.name);
                let style = sel_style(theme.warning).add_modifier(Modifier::BOLD);
                (bracket, style)
            }
        };

        spans.push(Span::styled(bracket, style));
    }

    let graph_width = max_graph_cols * COL_SPACING + 2;
    let hash_width = 8;
    let author_date_prefix = format!(" — {}", node.author);
    let relative_date = format_relative_time(node.timestamp);
    let author_date_suffix = format!(" {}", relative_date);
    let overhead =
        graph_width + hash_width + refs_width + author_date_prefix.len() + author_date_suffix.len();
    let max_message_width = (available_width as usize).saturating_sub(overhead);

    let display_message = if node.message.len() > max_message_width && max_message_width > 3 {
        format!("{}…", &node.message[..max_message_width.saturating_sub(1)])
    } else {
        node.message.clone()
    };

    spans.push(Span::styled(
        display_message,
        sel_style(if is_selected {
            theme.selection_fg
        } else {
            theme.text_normal
        }),
    ));
    spans.push(Span::styled(
        author_date_prefix,
        sel_style(theme.text_secondary),
    ));
    spans.push(Span::styled(
        author_date_suffix,
        sel_style(theme.text_secondary).add_modifier(Modifier::DIM),
    ));

    Line::from(spans)
}

pub(super) fn build_connection_line(connection: &ConnectionRow) -> Line<'static> {
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

            if col < num_cols - 1 {
                let needs_horizontal_right = col + 1 < num_cols
                    && connection.cells[col + 1]
                        .as_ref()
                        .is_some_and(|c| c.edge_type == EdgeType::Horizontal);
                let needs_horizontal_left = col > 0
                    && connection.cells[col - 1]
                        .as_ref()
                        .is_some_and(|c| c.edge_type == EdgeType::Horizontal);

                if needs_horizontal_right || needs_horizontal_left {
                    spans.push(Span::styled("─", Style::default().fg(color)));
                } else {
                    spans.push(Span::raw(" "));
                }
            }
        } else {
            let left_is_horizontal = col > 0
                && connection
                    .cells
                    .get(col - 1)
                    .and_then(|c| c.as_ref())
                    .is_some_and(|c| {
                        matches!(
                            c.edge_type,
                            EdgeType::Horizontal | EdgeType::MergeFromRight | EdgeType::Cross
                        )
                    });

            let right_is_horizontal = col + 1 < connection.cells.len()
                && connection
                    .cells
                    .get(col + 1)
                    .and_then(|c| c.as_ref())
                    .is_some_and(|c| {
                        matches!(
                            c.edge_type,
                            EdgeType::Horizontal
                                | EdgeType::ForkRight
                                | EdgeType::ForkLeft
                                | EdgeType::Cross
                        )
                    });

            if left_is_horizontal && right_is_horizontal {
                if let Some(idx) = find_horizontal_color_bounded(col, connection) {
                    let color = get_branch_color(idx);
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

pub(super) fn find_horizontal_color_bounded(
    col: usize,
    connection: &ConnectionRow,
) -> Option<usize> {
    for c in (0..col).rev() {
        match &connection.cells[c] {
            Some(cell) if cell.edge_type == EdgeType::Horizontal => return Some(cell.color_index),
            Some(cell)
                if matches!(
                    cell.edge_type,
                    EdgeType::MergeFromRight | EdgeType::MergeFromLeft
                ) =>
            {
                return Some(cell.color_index)
            }
            Some(_) => break,
            None => continue,
        }
    }

    for c in (col + 1)..connection.cells.len() {
        match &connection.cells[c] {
            Some(cell) if cell.edge_type == EdgeType::Horizontal => return Some(cell.color_index),
            Some(cell) if matches!(cell.edge_type, EdgeType::ForkRight | EdgeType::ForkLeft) => {
                return Some(cell.color_index)
            }
            Some(_) => break,
            None => continue,
        }
    }

    None
}

pub(super) fn get_branch_color(index: usize) -> Color {
    branch_color(index)
}
