use crate::error::Result;
use crate::state::{AppState, StagingFocus, ViewMode};
use crate::utils::flash_success;

use super::refresh_staging;

pub(super) fn handle_start_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.is_committing = true;
        state.staging_state.focus = StagingFocus::CommitMessage;
        state.staging_state.reset_commit_editing();
    }
    Ok(())
}

pub(super) fn handle_confirm_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging && !state.staging_state.commit_message.is_empty() {
        let message = state.staging_state.commit_message.clone();

        if state.staging_state.is_amending {
            crate::git::commit::amend_commit(&state.repo.repo, &message)?;
            state.set_flash_message(flash_success("Commit amendé"));
        } else {
            crate::git::commit::create_commit(&state.repo.repo, &message)?;
            state.set_flash_message(flash_success("Commit créé"));
        }

        state.staging_state.is_committing = false;
        state.staging_state.is_amending = false;
        state.staging_state.commit_message.clear();
        state.staging_state.reset_commit_editing();
        state.staging_state.focus = StagingFocus::Unstaged;

        state.mark_dirty();
        refresh_staging(state)?;
    }
    Ok(())
}

pub(super) fn handle_cancel_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.is_committing = false;
        state.staging_state.is_amending = false;
        state.staging_state.commit_message.clear();
        state.staging_state.reset_commit_editing();
        state.staging_state.focus = StagingFocus::Unstaged;
    }
    Ok(())
}
