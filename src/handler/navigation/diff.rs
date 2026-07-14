use crate::state::{AppState, ViewMode};

/// Hauteur visible estimée du panneau diff (en lignes).
pub(super) const DIFF_VISIBLE_HEIGHT_ESTIMATE: usize = 20;

pub(super) fn handle_scroll_diff_up(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Staging => {
            state.staging_state.diff_selected_line =
                state.staging_state.diff_selected_line.saturating_sub(1);
            keep_staging_diff_selection_visible(state);
        }
        ViewMode::ProjectTree => {
            state.project_tree_state.diff_scroll_offset = state
                .project_tree_state
                .diff_scroll_offset
                .saturating_sub(1);
        }
        _ => state.graph_view.scroll_diff_up(),
    }
}

pub(super) fn handle_scroll_diff_down(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Staging => {
            let last_line = staging_diff_last_line(state);
            state.staging_state.diff_selected_line = state
                .staging_state
                .diff_selected_line
                .saturating_add(1)
                .min(last_line);
            keep_staging_diff_selection_visible(state);
        }
        ViewMode::ProjectTree => state.project_tree_state.diff_scroll_offset += 1,
        _ => state.graph_view.scroll_diff_down(),
    }
}

pub(super) fn handle_scroll_diff_page_up(state: &mut AppState) {
    let page_size = DIFF_VISIBLE_HEIGHT_ESTIMATE / 2;
    match state.view_mode {
        ViewMode::Staging => {
            state.staging_state.diff_selected_line = state
                .staging_state
                .diff_selected_line
                .saturating_sub(page_size);
            keep_staging_diff_selection_visible(state);
        }
        ViewMode::ProjectTree => {
            state.project_tree_state.diff_scroll_offset = state
                .project_tree_state
                .diff_scroll_offset
                .saturating_sub(page_size);
        }
        _ => state.graph_view.scroll_diff_page_up(),
    }
}

pub(super) fn handle_scroll_diff_page_down(state: &mut AppState) {
    let page_size = DIFF_VISIBLE_HEIGHT_ESTIMATE / 2;
    match state.view_mode {
        ViewMode::Staging => {
            let last_line = staging_diff_last_line(state);
            state.staging_state.diff_selected_line = state
                .staging_state
                .diff_selected_line
                .saturating_add(page_size)
                .min(last_line);
            keep_staging_diff_selection_visible(state);
        }
        ViewMode::ProjectTree => state.project_tree_state.diff_scroll_offset += page_size,
        _ => state.graph_view.scroll_diff_page_down(),
    }
}

pub(super) fn handle_scroll_diff_top(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Staging => {
            state.staging_state.diff_selected_line = 0;
            state.staging_state.diff_scroll = 0;
        }
        ViewMode::ProjectTree => state.project_tree_state.diff_scroll_offset = 0,
        _ => state.graph_view.scroll_diff_top(),
    }
}

pub(super) fn handle_scroll_diff_bottom(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Staging => {
            state.staging_state.diff_selected_line = staging_diff_last_line(state);
            keep_staging_diff_selection_visible(state);
        }
        ViewMode::ProjectTree => {
            state.project_tree_state.diff_scroll_offset =
                state.project_tree_state.diff_total_lines.saturating_sub(1);
        }
        _ => state.graph_view.scroll_diff_bottom(),
    }
}

pub(super) fn handle_scroll_diff_left(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Staging => {
            state.staging_state.diff_horizontal_offset =
                state.staging_state.diff_horizontal_offset.saturating_sub(1);
        }
        ViewMode::ProjectTree => {
            state.project_tree_state.diff_horizontal_offset = state
                .project_tree_state
                .diff_horizontal_offset
                .saturating_sub(1);
        }
        _ => state.graph_view.scroll_diff_left(),
    }
}

pub(super) fn handle_scroll_diff_right(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Staging => state.staging_state.diff_horizontal_offset += 1,
        ViewMode::ProjectTree => state.project_tree_state.diff_horizontal_offset += 1,
        _ => state.graph_view.scroll_diff_right(),
    }
}

fn staging_diff_last_line(state: &AppState) -> usize {
    state
        .staging_state
        .current_diff
        .as_ref()
        .map_or(0, |diff| diff.lines.len().saturating_sub(1))
}

fn keep_staging_diff_selection_visible(state: &mut AppState) {
    let selected = state.staging_state.diff_selected_line;
    let scroll = &mut state.staging_state.diff_scroll;
    if selected < *scroll {
        *scroll = selected;
    } else if selected >= scroll.saturating_add(DIFF_VISIBLE_HEIGHT_ESTIMATE) {
        *scroll = selected.saturating_sub(DIFF_VISIBLE_HEIGHT_ESTIMATE - 1);
    }
}

pub(super) fn handle_scroll_stash_diff_up(state: &mut AppState) {
    if state.branches_view_state.stash_diff_scroll > 0 {
        state.branches_view_state.stash_diff_scroll -= 1;
    }
}

pub(super) fn handle_scroll_stash_diff_down(state: &mut AppState) {
    state.branches_view_state.stash_diff_scroll += 1;
}

pub fn load_commit_file_diff(state: &mut AppState) {
    if let Some(commit_oid) = state.selected_commit().map(|commit| commit.oid) {
        let file_index = state.graph_view.file_selected_index;
        if let Some(file) = state.graph_view.commit_files.get(file_index) {
            let path = file.path.clone();
            let cache_key = crate::state::cache::DiffCacheKey::new(commit_oid, &path);

            if let Some(cached_diff) = state.diff_cache.get(&cache_key) {
                state.graph_view.set_file_diff(Some(cached_diff));
            } else {
                match state.repo.file_diff(commit_oid, &path) {
                    Ok(diff) => {
                        let diff = std::sync::Arc::new(diff);
                        state.diff_cache.put(cache_key, diff.clone());
                        state.graph_view.set_file_diff(Some(diff));
                    }
                    Err(e) => {
                        state.graph_view.clear_file_diff();
                        state.set_flash_message(crate::utils::flash_error("chargement diff", e));
                    }
                }
            }
            return;
        }
    }
    state.graph_view.clear_file_diff();
}
