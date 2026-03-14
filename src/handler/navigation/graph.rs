use crate::state::{AppState, FocusPanel, StagingFocus, ViewMode};

use super::{handle_blame_navigation, handle_branches_navigation, load_commit_file_diff};

pub(super) fn handle_move_up(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            state.graph_view.select_previous();
            refresh_commit_file_data(state);
        }
        ViewMode::Staging => handle_staging_navigation(state, -1),
        ViewMode::Branches => handle_branches_navigation(state, -1),
        ViewMode::Blame => handle_blame_navigation(state, -1),
        _ => {}
    }
}

pub(super) fn handle_move_down(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            if !state.graph_view.is_empty() {
                state.graph_view.select_next();
                refresh_commit_file_data(state);
                let _ = crate::handler::dispatcher::maybe_load_more_history(state);
            }
        }
        ViewMode::Staging => handle_staging_navigation(state, 1),
        ViewMode::Branches => handle_branches_navigation(state, 1),
        ViewMode::Blame => handle_blame_navigation(state, 1),
        _ => {}
    }
}

pub(super) fn handle_page_up(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, -10);
            state.schedule_refresh();
        }
        _ => {
            if !state.graph_view.is_empty() {
                state.graph_view.page_up();
                refresh_commit_file_data(state);
            }
        }
    }
}

pub(super) fn handle_page_down(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, 10);
            state.schedule_refresh();
        }
        _ => {
            if !state.graph_view.is_empty() {
                state.graph_view.page_down();
                refresh_commit_file_data(state);
                let _ = crate::handler::dispatcher::maybe_load_more_history(state);
            }
        }
    }
}

pub(super) fn handle_go_top(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, -10000);
            state.schedule_refresh();
        }
        _ => {
            state.graph_view.go_top();
            refresh_commit_file_data(state);
        }
    }
}

pub(super) fn handle_go_bottom(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, 10000);
            state.schedule_refresh();
        }
        _ => {
            if !state.graph_view.is_empty() {
                let _ = crate::handler::dispatcher::load_all_history(state);
                state.graph_view.go_bottom();
                refresh_commit_file_data(state);
            }
        }
    }
}

pub(super) fn handle_switch_panel(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            state.focus = match state.focus {
                FocusPanel::Graph => FocusPanel::BottomLeft,
                FocusPanel::BottomLeft => FocusPanel::Graph,
                FocusPanel::BottomRight => FocusPanel::BottomLeft,
            };
            if state.focus == FocusPanel::BottomLeft {
                load_commit_file_diff(state);
            }
        }
        ViewMode::Staging => {
            state.staging_state.focus = match state.staging_state.focus {
                StagingFocus::Unstaged => StagingFocus::Staged,
                StagingFocus::Staged => StagingFocus::Diff,
                StagingFocus::Diff => StagingFocus::CommitMessage,
                StagingFocus::CommitMessage => StagingFocus::Unstaged,
            };
        }
        _ => {}
    }
}

pub(super) fn handle_file_up(state: &mut AppState) {
    state.graph_view.select_previous_file();
    load_commit_file_diff(state);
}

pub(super) fn handle_file_down(state: &mut AppState) {
    state.graph_view.select_next_file();
    load_commit_file_diff(state);
}

pub(super) fn handle_back_to_graph(state: &mut AppState) {
    if state.view_mode == ViewMode::Graph {
        state.focus = FocusPanel::Graph;
    }
}

pub fn refresh_commit_file_data(state: &mut AppState) {
    state.refresh_commit_files();
    if !state.graph_view.commit_files.is_empty() {
        load_commit_file_diff(state);
    } else {
        state.graph_view.clear_file_diff();
    }
}

fn handle_staging_navigation(state: &mut AppState, direction: i32) {
    match state.staging_state.focus {
        StagingFocus::Unstaged => {
            let max = state.staging_state.unstaged_files().len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.staging_state.unstaged_selected() + 1).min(max - 1)
                } else {
                    state.staging_state.unstaged_selected().saturating_sub(1)
                };
                state.staging_state.set_unstaged_selected(new_idx);
                crate::handler::staging::load_staging_diff(state);
            }
        }
        StagingFocus::Staged => {
            let max = state.staging_state.staged_files().len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.staging_state.staged_selected() + 1).min(max - 1)
                } else {
                    state.staging_state.staged_selected().saturating_sub(1)
                };
                state.staging_state.set_staged_selected(new_idx);
                crate::handler::staging::load_staging_diff(state);
            }
        }
        StagingFocus::Diff => {
            if direction > 0 {
                state.staging_state.diff_scroll += 1;
            } else if state.staging_state.diff_scroll > 0 {
                state.staging_state.diff_scroll -= 1;
            }
        }
        _ => {}
    }
}
