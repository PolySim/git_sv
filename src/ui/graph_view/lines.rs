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
    let is_head = node
        .refs
        .iter()
        .any(|reference| reference.ref_type == RefType::Head);

    let mut spans: Vec<Span<'static>> = Vec::new();
    let num_cols = row.cells.len().max(node.column + 1);

    for col in 0..num_cols {
        if col == node.column {
            let symbol = if is_head {
                "◆"
            } else if node.parents.len() > 1 {
                "○"
            } else {
                "●"
            };
            spans.push(Span::styled(
                symbol.to_string(),
                Style::default()
                    .fg(if is_head { theme.success } else { commit_color })
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

    spans.push(Span::styled(
        if is_selected { " ▶ " } else { "   " },
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    ));

    let content_style = |base_fg: Color| -> Style {
        let style = Style::default().fg(base_fg);
        if is_selected {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    };

    let hash = node.oid.to_string();
    let short_hash = if hash.len() >= 7 { &hash[..7] } else { &hash };
    spans.push(Span::styled(
        format!("{} ", short_hash),
        content_style(theme.commit_hash),
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

    let metadata = build_metadata(node.author.as_str(), node.timestamp, available_width);
    let metadata_width = metadata.as_ref().map_or(0, |(author, date)| {
        display_width(author) + display_width(date)
    });
    let occupied_width = spans_width(&spans) + metadata_width;
    let refs_budget = (available_width as usize)
        .saturating_sub(occupied_width + MIN_MESSAGE_WIDTH)
        .min(MAX_REFS_WIDTH);
    push_ref_badges(&mut spans, &sorted_refs, refs_budget, node.color_index);

    let max_message_width =
        (available_width as usize).saturating_sub(spans_width(&spans) + metadata_width);
    let display_message = truncate_text(
        node.message.lines().next().unwrap_or_default(),
        max_message_width,
    );

    spans.push(Span::styled(
        display_message,
        content_style(if is_selected {
            theme.selection_fg
        } else {
            theme.text_normal
        }),
    ));

    if let Some((author, date)) = metadata {
        spans.push(Span::styled(author, content_style(theme.text_secondary)));
        spans.push(Span::styled(
            date,
            content_style(theme.text_secondary).add_modifier(Modifier::DIM),
        ));
    }

    let line_width = spans_width(&spans);
    if line_width < available_width as usize {
        spans.push(Span::raw(" ".repeat(available_width as usize - line_width)));
    }

    if is_selected {
        for span in &mut spans {
            if span.style.bg.is_none() {
                span.style = span.style.bg(theme.selection_bg);
            }
        }
    }

    Line::from(spans)
}

const MIN_MESSAGE_WIDTH: usize = 16;
const MAX_REFS_WIDTH: usize = 40;

fn push_ref_badges(
    spans: &mut Vec<Span<'static>>,
    refs: &[&crate::git::graph::RefInfo],
    width_budget: usize,
    color_index: usize,
) {
    let theme = current_theme();
    let mut used = 0;
    let mut hidden = 0;

    for reference in refs {
        let (label, style) = match reference.ref_type {
            RefType::Head => (
                format!(" HEAD:{} ", reference.name),
                Style::default()
                    .fg(theme.background)
                    .bg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            RefType::LocalBranch => (
                format!(" {} ", reference.name),
                Style::default()
                    .fg(theme.background)
                    .bg(get_branch_color(color_index))
                    .add_modifier(Modifier::BOLD),
            ),
            RefType::Tag => (
                format!(" tag:{} ", reference.name),
                Style::default()
                    .fg(theme.background)
                    .bg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            RefType::RemoteBranch => (
                format!(" remote:{} ", reference.name),
                Style::default()
                    .fg(theme.text_secondary)
                    .add_modifier(Modifier::DIM),
            ),
        };
        let label_width = display_width(&label);
        if used + label_width < width_budget {
            spans.push(Span::styled(label, style));
            spans.push(Span::raw(" "));
            used += label_width + 1;
        } else {
            hidden += 1;
        }
    }

    if hidden > 0 {
        let summary = format!(" +{} ", hidden);
        if used + display_width(&summary) <= width_budget {
            spans.push(Span::styled(
                summary,
                Style::default()
                    .fg(theme.text_secondary)
                    .add_modifier(Modifier::DIM),
            ));
        }
    }
}

fn build_metadata(author: &str, timestamp: i64, available_width: u16) -> Option<(String, String)> {
    if available_width < 58 {
        return None;
    }

    if available_width < 96 {
        let author = truncate_text(author, 10);
        let relative_date = truncate_text(&format_relative_time(timestamp), 12);
        return Some((
            format!(" │ {:<10}", author),
            format!(" · {:>12}", relative_date),
        ));
    }

    let author = truncate_text(author, 14);
    let relative_date = truncate_text(&format_relative_time(timestamp), 16);
    Some((
        format!(" │ {:<14}", author),
        format!(" · {:>16}", relative_date),
    ))
}

fn truncate_text(value: &str, max_width: usize) -> String {
    let width = display_width(value);
    if width <= max_width {
        return value.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let mut truncated: String = value.chars().take(max_width - 1).collect();
    truncated.push('…');
    truncated
}

fn display_width(value: &str) -> usize {
    value.chars().count()
}

fn spans_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(Span::width).sum()
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
