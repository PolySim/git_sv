use crate::state::{AppState, ViewMode};

/// Hauteur visible estimée du panneau diff (en lignes).
pub(super) const DIFF_VISIBLE_HEIGHT_ESTIMATE: usize = 20;

pub(super) fn handle_scroll_diff_up(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        if state.staging_state.diff_scroll > 0 {
            state.staging_state.diff_scroll -= 1;
        }
    } else {
        state.graph_view.scroll_diff_up();
    }
}

pub(super) fn handle_scroll_diff_down(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.diff_scroll += 1;
    } else {
        state.graph_view.scroll_diff_down();
    }
}

pub(super) fn handle_scroll_diff_page_up(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        let page_size = DIFF_VISIBLE_HEIGHT_ESTIMATE / 2;
        state.staging_state.diff_scroll = state.staging_state.diff_scroll.saturating_sub(page_size);
    } else {
        state.graph_view.scroll_diff_page_up();
    }
}

pub(super) fn handle_scroll_diff_page_down(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        let page_size = DIFF_VISIBLE_HEIGHT_ESTIMATE / 2;
        state.staging_state.diff_scroll += page_size;
    } else {
        state.graph_view.scroll_diff_page_down();
    }
}

pub(super) fn handle_scroll_diff_top(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.diff_scroll = 0;
    } else {
        state.graph_view.scroll_diff_top();
    }
}

pub(super) fn handle_scroll_diff_bottom(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.diff_scroll = usize::MAX / 4;
    } else {
        state.graph_view.scroll_diff_bottom();
    }
}

pub(super) fn handle_scroll_diff_left(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        if state.staging_state.diff_horizontal_offset > 0 {
            state.staging_state.diff_horizontal_offset -= 1;
        }
    } else {
        state.graph_view.scroll_diff_left();
    }
}

pub(super) fn handle_scroll_diff_right(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.diff_horizontal_offset += 1;
    } else {
        state.graph_view.scroll_diff_right();
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
    if let Some(commit) = state.selected_commit() {
        let file_index = state.graph_view.file_selected_index;
        if let Some(file) = state.graph_view.commit_files.get(file_index) {
            let diff = state.repo.file_diff(commit.oid, &file.path).ok();
            state.graph_view.set_file_diff(diff);
            return;
        }
    }
    state.graph_view.clear_file_diff();
}
