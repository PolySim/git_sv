use crate::state::{AppState, BranchesSection};

pub(super) fn handle_branches_navigation(state: &mut AppState, direction: i32) {
    match state.branches_view_state.section {
        BranchesSection::Branches => {
            let has_local = !state.branches_view_state.local_branches.is_empty();
            let has_remote = state.branches_view_state.show_remote
                && !state.branches_view_state.remote_branches.is_empty();

            if has_local || has_remote {
                if direction > 0 {
                    state.branches_view_state.select_next();
                } else {
                    state.branches_view_state.select_prev();
                }
            }
        }
        BranchesSection::Worktrees => {
            let max = state.branches_view_state.worktrees.len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.branches_view_state.worktree_selected() + 1).min(max - 1)
                } else {
                    state
                        .branches_view_state
                        .worktree_selected()
                        .saturating_sub(1)
                };
                state.branches_view_state.set_worktree_selected(new_idx);
            }
        }
        BranchesSection::Stashes => {
            let max = state.branches_view_state.stashes.len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.branches_view_state.stash_selected() + 1).min(max - 1)
                } else {
                    state.branches_view_state.stash_selected().saturating_sub(1)
                };
                state.branches_view_state.set_stash_selected(new_idx);
                state.branches_view_state.stash_file_selected = 0;
                state.branches_view_state.stash_file_diff = None;
                state.branches_view_state.stash_diff_scroll = 0;
                let _ = crate::handler::branch::load_stash_file_diff(state);
            }
        }
    }
}
