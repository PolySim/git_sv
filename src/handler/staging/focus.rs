use crate::error::Result;
use crate::state::{AppState, StagingFocus, ViewMode};

use super::load_staging_diff;

pub(super) fn handle_focus_unstaged(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.focus = StagingFocus::Unstaged;
        state.staging_state.last_file_focus = StagingFocus::Unstaged;
        load_staging_diff(state);
    }
    Ok(())
}

pub(super) fn handle_focus_staged(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.focus = StagingFocus::Staged;
        state.staging_state.last_file_focus = StagingFocus::Staged;
        load_staging_diff(state);
    }
    Ok(())
}

pub(super) fn handle_select_unstaged(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.set_unstaged_selected(index);
        state.staging_state.focus = StagingFocus::Unstaged;
        state.staging_state.last_file_focus = StagingFocus::Unstaged;
        load_staging_diff(state);
    }
    Ok(())
}

pub(super) fn handle_select_staged(state: &mut AppState, index: usize) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.set_staged_selected(index);
        state.staging_state.focus = StagingFocus::Staged;
        state.staging_state.last_file_focus = StagingFocus::Staged;
        load_staging_diff(state);
    }
    Ok(())
}

pub(super) fn handle_switch_focus(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.focus = match state.staging_state.focus {
            StagingFocus::Unstaged => {
                state.staging_state.last_file_focus = StagingFocus::Staged;
                StagingFocus::Staged
            }
            StagingFocus::Staged => {
                state.staging_state.last_file_focus = StagingFocus::Unstaged;
                StagingFocus::Unstaged
            }
            StagingFocus::Diff => state.staging_state.last_file_focus,
            StagingFocus::CommitMessage => StagingFocus::Unstaged,
        };
        load_staging_diff(state);
    }
    Ok(())
}

pub(super) fn handle_focus_diff(state: &mut AppState) -> Result<()> {
    if state.view_mode != ViewMode::Staging {
        return Ok(());
    }

    if matches!(
        state.staging_state.focus,
        StagingFocus::Unstaged | StagingFocus::Staged
    ) {
        state.staging_state.last_file_focus = state.staging_state.focus;
        state.staging_state.focus = StagingFocus::Diff;
        load_staging_diff(state);
    }

    Ok(())
}
