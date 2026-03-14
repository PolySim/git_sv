use crate::error::Result;
use crate::state::{AppState, BranchesFocus, SelectedBranch, ViewMode};

pub(super) fn handle_select_local_branch(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.selected_branch = Some(SelectedBranch::Local(index));
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

pub(super) fn handle_select_remote_branch(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.selected_branch = Some(SelectedBranch::Remote(index));
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

pub(super) fn handle_select_worktree(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.set_worktree_selected(index);
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

pub(super) fn handle_select_stash(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.set_stash_selected(index);
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

pub(super) fn handle_focus_list(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::List;
    }
    Ok(())
}

pub(super) fn handle_focus_detail(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::Detail;
    }
    Ok(())
}
