//! Handler pour les actions de navigation.

use crate::error::Result;
use crate::state::{AppState, ViewMode, FocusPanel, StagingFocus, BranchesSection};
use crate::state::action::NavigationAction;
use super::traits::{ActionHandler, HandlerContext};

/// Handler pour la navigation dans les listes.
pub struct NavigationHandler;

impl ActionHandler for NavigationHandler {
    type Action = NavigationAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: NavigationAction) -> Result<()> {
        match action {
            NavigationAction::MoveUp => handle_move_up(ctx.state),
            NavigationAction::MoveDown => handle_move_down(ctx.state),
            NavigationAction::PageUp => handle_page_up(ctx.state),
            NavigationAction::PageDown => handle_page_down(ctx.state),
            NavigationAction::GoTop => handle_go_top(ctx.state),
            NavigationAction::GoBottom => handle_go_bottom(ctx.state),
            NavigationAction::SwitchPanel => handle_switch_panel(ctx.state),
            NavigationAction::ScrollDiffUp => handle_scroll_diff_up(ctx.state),
            NavigationAction::ScrollDiffDown => handle_scroll_diff_down(ctx.state),
            NavigationAction::FileUp => handle_file_up(ctx.state),
            NavigationAction::FileDown => handle_file_down(ctx.state),
        }

        Ok(())
    }
}

fn handle_move_up(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            if state.show_branch_panel {
                if state.branch_selected > 0 {
                    state.branch_selected -= 1;
                }
            } else if state.selected_index > 0 {
                state.selected_index -= 1;
                state.graph_state.select(Some(state.selected_index * 2));
                state.sync_legacy_selection();
            }
        }
        ViewMode::Staging => {
            handle_staging_navigation(state, -1);
        }
        ViewMode::Branches => {
            handle_branches_navigation(state, -1);
        }
        ViewMode::Blame => {
            handle_blame_navigation(state, -1);
        }
        _ => {}
    }
}

fn handle_move_down(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            if state.show_branch_panel {
                if state.branch_selected + 1 < state.branches.len() {
                    state.branch_selected += 1;
                }
            } else if state.selected_index + 1 < state.graph.len() {
                state.selected_index += 1;
                state.graph_state.select(Some(state.selected_index * 2));
                state.sync_legacy_selection();
            }
        }
        ViewMode::Staging => {
            handle_staging_navigation(state, 1);
        }
        ViewMode::Branches => {
            handle_branches_navigation(state, 1);
        }
        ViewMode::Blame => {
            handle_blame_navigation(state, 1);
        }
        _ => {}
    }
}

fn handle_page_up(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, -10);
        }
        _ => {
            if !state.show_branch_panel && !state.graph.is_empty() {
                let page_size = 10;
                state.selected_index = state.selected_index.saturating_sub(page_size);
                state.graph_state.select(Some(state.selected_index * 2));
                state.sync_legacy_selection();
            }
        }
    }
}

fn handle_page_down(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, 10);
        }
        _ => {
            if !state.show_branch_panel && !state.graph.is_empty() {
                let page_size = 10;
                state.selected_index = (state.selected_index + page_size).min(state.graph.len() - 1);
                state.graph_state.select(Some(state.selected_index * 2));
                state.sync_legacy_selection();
            }
        }
    }
}

fn handle_go_top(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, -10000);
        }
        _ => {
            if !state.show_branch_panel {
                state.selected_index = 0;
                state.graph_state.select(Some(0));
                state.sync_legacy_selection();
            }
        }
    }
}

fn handle_go_bottom(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, 10000);
        }
        _ => {
            if !state.show_branch_panel && !state.graph.is_empty() {
                state.selected_index = state.graph.len() - 1;
                state.graph_state.select(Some(state.selected_index * 2));
                state.sync_legacy_selection();
            }
        }
    }
}

fn handle_switch_panel(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            state.focus = match state.focus {
                FocusPanel::Graph => FocusPanel::BottomLeft,
                FocusPanel::BottomLeft => FocusPanel::BottomRight,
                FocusPanel::BottomRight => FocusPanel::Graph,
                // Legacy variants
                FocusPanel::Files => FocusPanel::BottomRight,
                FocusPanel::Detail => FocusPanel::Graph,
            };
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

fn handle_scroll_diff_up(state: &mut AppState) {
    if state.diff_scroll_offset > 0 {
        state.diff_scroll_offset -= 1;
    }
}

fn handle_scroll_diff_down(state: &mut AppState) {
    state.diff_scroll_offset += 1;
}

fn handle_file_up(state: &mut AppState) {
    if state.file_selected_index > 0 {
        state.file_selected_index -= 1;
        state.graph_view.file_selected_index = state.file_selected_index;
    }
}

fn handle_file_down(state: &mut AppState) {
    if state.file_selected_index + 1 < state.commit_files.len() {
        state.file_selected_index += 1;
        state.graph_view.file_selected_index = state.file_selected_index;
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

fn handle_branches_navigation(state: &mut AppState, direction: i32) {
    match state.branches_view_state.section {
        BranchesSection::Branches => {
            let local_count = state.branches_view_state.local_branches.len();
            let remote_count = state.branches_view_state.remote_branches.len();
            let show_remote = state.branches_view_state.show_remote;

            let max = if show_remote && remote_count > 0 {
                local_count + remote_count
            } else {
                local_count
            };

            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.branches_view_state.branch_selected() + 1).min(max - 1)
                } else {
                    state.branches_view_state.branch_selected().saturating_sub(1)
                };
                state.branches_view_state.set_branch_selected(new_idx);
            }
        }
        BranchesSection::Worktrees => {
            let max = state.branches_view_state.worktrees.len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.branches_view_state.worktree_selected() + 1).min(max - 1)
                } else {
                    state.branches_view_state.worktree_selected().saturating_sub(1)
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
            }
        }
    }
}

fn handle_blame_navigation(state: &mut AppState, delta: i32) {
    if let Some(ref mut blame_state) = state.blame_state {
        let line_count = if let Some(ref blame) = blame_state.blame {
            blame.lines.len()
        } else {
            0
        };

        let new_idx = if delta >= 0 {
            (blame_state.selected_line + delta as usize).min(line_count.saturating_sub(1))
        } else {
            blame_state.selected_line.saturating_sub((-delta) as usize)
        };
        blame_state.selected_line = new_idx;
    }
}
