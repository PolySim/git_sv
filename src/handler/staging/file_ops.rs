use crate::error::Result;
use crate::state::{AppState, ViewMode};

use super::refresh_staging;

pub(super) fn handle_stage_file(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        if let Some(file) = state
            .staging_state
            .unstaged_files()
            .get(state.staging_state.unstaged_selected())
        {
            crate::git::commit::stage_file(&state.repo.repo, &file.path)?;
            state.mark_dirty();
            refresh_staging(state)?;
        }
    }
    Ok(())
}

pub(super) fn handle_unstage_file(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        if let Some(file) = state
            .staging_state
            .staged_files()
            .get(state.staging_state.staged_selected())
        {
            crate::git::commit::unstage_file(&state.repo.repo, &file.path)?;
            state.mark_dirty();
            refresh_staging(state)?;
        }
    }
    Ok(())
}

pub(super) fn handle_stage_all(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        crate::git::commit::stage_all(&state.repo.repo)?;
        state.mark_dirty();
        refresh_staging(state)?;
    }
    Ok(())
}

pub(super) fn handle_unstage_all(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        crate::git::commit::unstage_all(&state.repo.repo)?;
        state.mark_dirty();
        refresh_staging(state)?;
    }
    Ok(())
}

pub(super) fn handle_discard_file(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Staging {
        if let Some(file) = state
            .staging_state
            .unstaged_files()
            .get(state.staging_state.unstaged_selected())
        {
            let path = file.path.clone();
            state.open_confirmation(ConfirmAction::DiscardFile(path));
        }
    }
    Ok(())
}

pub(super) fn handle_discard_all(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Staging {
        state.open_confirmation(ConfirmAction::DiscardAll);
    }
    Ok(())
}
