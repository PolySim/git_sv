use std::collections::HashMap;

use git2::Oid;

use super::{ColumnState, EdgeType, GraphCell, RefInfo};

pub(super) fn build_commit_cells(
    commit_col: usize,
    active_columns: &[ColumnState],
    merged_columns: &[usize],
) -> Vec<Option<GraphCell>> {
    let num_cols = active_columns.len().max(commit_col + 1);
    let mut cells: Vec<Option<GraphCell>> = Vec::with_capacity(num_cols);

    for col in 0..num_cols {
        if col == commit_col {
            cells.push(None);
        } else if col < active_columns.len() && active_columns[col].expected_oid.is_some() {
            cells.push(Some(GraphCell::new(
                EdgeType::Vertical,
                active_columns[col].color_index,
            )));
        } else {
            cells.push(None);
        }
    }

    for &merged_col in merged_columns {
        if merged_col == commit_col {
            continue;
        }

        let (start, end) = if merged_col > commit_col {
            (commit_col + 1, merged_col)
        } else {
            (merged_col + 1, commit_col)
        };

        for cell in cells.iter_mut().take(end).skip(start) {
            match cell {
                Some(existing) if existing.edge_type() == EdgeType::Vertical => {
                    *existing = GraphCell::new(EdgeType::Cross, existing.color_index());
                }
                Some(_) => {}
                None => {
                    *cell = Some(GraphCell::new(
                        EdgeType::Horizontal,
                        active_columns[commit_col].color_index,
                    ));
                }
            }
        }

        cells[merged_col] = Some(GraphCell::new(
            if merged_col > commit_col {
                EdgeType::MergeFromLeft
            } else {
                EdgeType::MergeFromRight
            },
            active_columns[merged_col].color_index,
        ));
    }

    cells
}

pub(super) fn assign_parent_columns(
    active_columns: &mut Vec<ColumnState>,
    commit_col: usize,
    ci: &crate::git::commit::CommitInfo,
    commit_color: usize,
) -> Vec<(usize, usize, usize)> {
    let mut assignments = Vec::new();

    for (i, &parent_oid) in ci.parents.iter().enumerate() {
        if i == 0 {
            if commit_col < active_columns.len() {
                active_columns[commit_col].expected_oid = Some(parent_oid);
                assignments.push((commit_col, commit_col, commit_color));
            } else {
                while active_columns.len() <= commit_col {
                    active_columns.push(ColumnState {
                        expected_oid: None,
                        color_index: 0,
                        branch_name: None,
                    });
                }
                active_columns[commit_col].expected_oid = Some(parent_oid);
                active_columns[commit_col].color_index = commit_color;
                assignments.push((commit_col, commit_col, commit_color));
            }
        } else if let Some(parent_col) = active_columns
            .iter()
            .position(|s| s.expected_oid == Some(parent_oid))
        {
            let merge_color = active_columns[parent_col].color_index;
            assignments.push((commit_col, parent_col, merge_color));
        } else {
            let parent_col = assign_new_column(active_columns, parent_oid);
            active_columns[parent_col].color_index = commit_color;
            assignments.push((commit_col, parent_col, commit_color));
        }
    }

    assignments
}

pub(super) fn build_connection_row(
    active_columns: &[ColumnState],
    parent_assignments: &[(usize, usize, usize)],
) -> super::ConnectionRow {
    let num_cols = active_columns.len();
    let mut cells: Vec<Option<GraphCell>> = vec![None; num_cols];

    for (col, state) in active_columns.iter().enumerate() {
        if state.expected_oid.is_some() {
            cells[col] = Some(GraphCell::new(EdgeType::Vertical, state.color_index));
        }
    }

    for &(from_col, to_col, _color) in parent_assignments {
        if from_col != to_col && to_col < cells.len() {
            cells[to_col] = None;
        }
    }

    for &(from_col, to_col, color) in parent_assignments {
        if from_col == to_col {
            continue;
        }

        if to_col > from_col {
            cells[from_col] = Some(GraphCell::new(EdgeType::MergeFromRight, color));

            for slot in cells.iter_mut().take(to_col).skip(from_col + 1) {
                if let Some(existing_cell) = slot.as_ref() {
                    if existing_cell.edge_type() == EdgeType::Vertical {
                        let existing_color = existing_cell.color_index();
                        *slot = Some(GraphCell::new(EdgeType::Cross, existing_color));
                        continue;
                    }
                }
                *slot = Some(GraphCell::new(EdgeType::Horizontal, color));
            }

            cells[to_col] = Some(GraphCell::new(EdgeType::ForkRight, color));
        } else {
            cells[from_col] = Some(GraphCell::new(EdgeType::MergeFromLeft, color));

            for slot in cells.iter_mut().take(from_col).skip(to_col + 1) {
                if let Some(existing_cell) = slot.as_ref() {
                    if existing_cell.edge_type() == EdgeType::Vertical {
                        let existing_color = existing_cell.color_index();
                        *slot = Some(GraphCell::new(EdgeType::Cross, existing_color));
                        continue;
                    }
                }
                *slot = Some(GraphCell::new(EdgeType::Horizontal, color));
            }

            cells[to_col] = Some(GraphCell::new(EdgeType::ForkLeft, color));
        }
    }

    super::ConnectionRow { cells }
}

pub(super) fn determine_color_index(
    column: usize,
    refs: &[RefInfo],
    branch_colors: &mut HashMap<String, usize>,
    next_color_index: &mut usize,
    active_columns: &[ColumnState],
) -> usize {
    if let Some(first_ref) = refs.first() {
        if let Some(&color) = branch_colors.get(&first_ref.name) {
            return color;
        }
        let color = *next_color_index;
        branch_colors.insert(first_ref.name.clone(), color);
        *next_color_index += 1;
        return color;
    }

    if column < active_columns.len() && (active_columns[column].color_index > 0 || column == 0) {
        return active_columns[column].color_index;
    }

    column
}

pub(super) fn determine_branch_name(
    column: usize,
    refs: &[RefInfo],
    active_columns: &[ColumnState],
) -> Option<String> {
    if let Some(first_ref) = refs.first() {
        let name = &first_ref.name;
        if let Some(stripped) = name.strip_prefix("refs/heads/") {
            return Some(stripped.to_string());
        }
        return Some(name.clone());
    }

    if column < active_columns.len() {
        return active_columns[column].branch_name.clone();
    }

    None
}

pub(super) fn find_or_assign_column(
    active_columns: &mut Vec<ColumnState>,
    oid: Oid,
) -> (usize, Vec<usize>) {
    let matching_columns = active_columns
        .iter()
        .enumerate()
        .filter_map(|(i, state)| (state.expected_oid == Some(oid)).then_some(i))
        .collect::<Vec<_>>();

    if let Some(&column) = matching_columns.first() {
        for &duplicate_col in matching_columns.iter().skip(1) {
            active_columns[duplicate_col].expected_oid = None;
            active_columns[duplicate_col].branch_name = None;
        }

        return (column, matching_columns.into_iter().skip(1).collect());
    }

    (assign_new_column(active_columns, oid), Vec::new())
}

pub(super) fn assign_new_column(active_columns: &mut Vec<ColumnState>, oid: Oid) -> usize {
    for (i, state) in active_columns.iter_mut().enumerate() {
        if state.expected_oid.is_none() {
            state.expected_oid = Some(oid);
            return i;
        }
    }

    active_columns.push(ColumnState {
        expected_oid: Some(oid),
        color_index: 0,
        branch_name: None,
    });
    active_columns.len() - 1
}
