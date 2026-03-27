use crate::error::Result;
use crate::git::stash::StashPushOutcome;
use crate::state::{AppState, ViewMode};
use crate::utils::flash_success;

use super::refresh_staging;

pub(super) fn handle_stash_selected_file(state: &mut AppState) -> Result<()> {
    if state.view_mode != ViewMode::Staging {
        return Ok(());
    }

    let Some(file) = state
        .staging_state
        .unstaged_files()
        .get(state.staging_state.unstaged_selected())
    else {
        state.set_flash_message("Aucun fichier non stagé sélectionné".to_string());
        return Ok(());
    };

    let path = file.path.clone();
    let is_untracked = file.status.contains(git2::Status::WT_NEW) && !file.is_staged();

    let message = format!("git_sv: stash partiel {}", path);

    let outcome = if is_untracked {
        crate::git::stash::stash_untracked_file(&state.repo_path, &path, Some(&message))?
    } else {
        crate::git::stash::stash_file(&state.repo_path, &path, Some(&message))?
    };

    match outcome {
        StashPushOutcome::Created => {
            state.set_flash_message(flash_success(format!(
                "Stash créé pour {} (index conservé)",
                path
            )));
            state.mark_dirty();
            refresh_staging(state)?;
        }
        StashPushOutcome::NoChanges => {
            state.set_flash_message(crate::utils::flash_error_message(format!(
                "aucun changement non stagé à stasher pour {}",
                path
            )));
        }
    }

    Ok(())
}

pub(super) fn handle_stash_unstaged_files(state: &mut AppState) -> Result<()> {
    if state.view_mode != ViewMode::Staging {
        return Ok(());
    }

    let message = "git_sv: stash des changements non stagés";

    match crate::git::stash::stash_unstaged_files(&state.repo_path, Some(message))? {
        StashPushOutcome::Created => {
            state.set_flash_message(flash_success(
                "Stash des changements non stagés créé (index conservé)",
            ));
            state.mark_dirty();
            refresh_staging(state)?;
        }
        StashPushOutcome::NoChanges => {
            state.set_flash_message(crate::utils::flash_error_message(
                "aucun changement non stagé à stasher",
            ));
        }
    }

    Ok(())
}
