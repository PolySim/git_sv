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

pub(super) fn handle_stage_hunk(state: &mut AppState) -> Result<()> {
    handle_partial_change(state, PartialChange::StageHunk)
}

pub(super) fn handle_unstage_hunk(state: &mut AppState) -> Result<()> {
    handle_partial_change(state, PartialChange::UnstageHunk)
}

pub(super) fn handle_stage_line(state: &mut AppState) -> Result<()> {
    handle_partial_change(state, PartialChange::StageLine)
}

pub(super) fn handle_unstage_line(state: &mut AppState) -> Result<()> {
    handle_partial_change(state, PartialChange::UnstageLine)
}

#[derive(Clone, Copy)]
enum PartialChange {
    StageHunk,
    UnstageHunk,
    StageLine,
    UnstageLine,
}

fn handle_partial_change(state: &mut AppState, operation: PartialChange) -> Result<()> {
    if state.view_mode != ViewMode::Staging {
        return Ok(());
    }

    let expected_focus = match operation {
        PartialChange::StageHunk | PartialChange::StageLine => crate::state::StagingFocus::Unstaged,
        PartialChange::UnstageHunk | PartialChange::UnstageLine => {
            crate::state::StagingFocus::Staged
        }
    };
    if state.staging_state.last_file_focus != expected_focus {
        state.set_flash_message("Action indisponible pour ce diff".to_string());
        return Ok(());
    }

    let Some(diff) = state.staging_state.current_diff.clone() else {
        state.set_flash_message("Aucun diff sélectionné".to_string());
        return Ok(());
    };
    let selected_line = state.staging_state.diff_selected_line;
    let result = match operation {
        PartialChange::StageHunk => diff.hunk_at_line(selected_line).map_or_else(
            || Err("Sélectionnez une ligne appartenant à un hunk".to_string()),
            |hunk| {
                crate::git::staging::stage_hunk(&state.repo.repo, &diff.path, hunk)
                    .map_err(|error| error.to_string())
            },
        ),
        PartialChange::UnstageHunk => diff.hunk_at_line(selected_line).map_or_else(
            || Err("Sélectionnez une ligne appartenant à un hunk".to_string()),
            |hunk| {
                crate::git::staging::unstage_hunk(&state.repo.repo, &diff.path, hunk)
                    .map_err(|error| error.to_string())
            },
        ),
        PartialChange::StageLine => diff.change_at_line(selected_line).map_or_else(
            || Err("Sélectionnez une ligne ajoutée ou supprimée".to_string()),
            |selection| {
                crate::git::staging::stage_line(&state.repo.repo, &diff.path, selection)
                    .map_err(|error| error.to_string())
            },
        ),
        PartialChange::UnstageLine => diff.change_at_line(selected_line).map_or_else(
            || Err("Sélectionnez une ligne ajoutée ou supprimée".to_string()),
            |selection| {
                crate::git::staging::unstage_line(&state.repo.repo, &diff.path, selection)
                    .map_err(|error| error.to_string())
            },
        ),
    };

    match result {
        Ok(()) => {
            let message = match operation {
                PartialChange::StageHunk => "Hunk indexé ✓",
                PartialChange::UnstageHunk => "Hunk désindexé ✓",
                PartialChange::StageLine => "Ligne indexée ✓",
                PartialChange::UnstageLine => "Ligne désindexée ✓",
            };
            state.mark_dirty();
            refresh_staging(state)?;
            state.set_flash_message(message.to_string());
        }
        Err(error) => state.set_flash_message(error),
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
