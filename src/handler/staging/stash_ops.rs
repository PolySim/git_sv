use crate::error::Result;
use crate::git::stash::StashPushOutcome;
use crate::state::{AppState, ViewMode};

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
        state.set_flash_message("Aucun fichier non stage selectionne".to_string());
        return Ok(());
    };

    let path = file.path.clone();

    if file.status.contains(git2::Status::WT_NEW) && !file.is_staged() {
        state.set_flash_message(
            "Stash fichier indisponible pour un fichier non suivi, utilisez Ctrl+S".to_string(),
        );
        return Ok(());
    }

    let message = format!("git_sv: stash partiel {}", path);

    match crate::git::stash::stash_file(&state.repo_path, &path, Some(&message))? {
        StashPushOutcome::Created => {
            state.set_flash_message(format!("Stash cree pour {} (index conserve) ✓", path));
            state.mark_dirty();
            refresh_staging(state)?;
        }
        StashPushOutcome::NoChanges => {
            state.set_flash_message(format!(
                "Aucun changement non stage a stasher pour {}",
                path
            ));
        }
    }

    Ok(())
}

pub(super) fn handle_stash_unstaged_files(state: &mut AppState) -> Result<()> {
    if state.view_mode != ViewMode::Staging {
        return Ok(());
    }

    let message = "git_sv: stash des changements non stages";

    match crate::git::stash::stash_unstaged_files(&state.repo_path, Some(message))? {
        StashPushOutcome::Created => {
            state.set_flash_message("Stash des changements non stages cree (index conserve) ✓");
            state.mark_dirty();
            refresh_staging(state)?;
        }
        StashPushOutcome::NoChanges => {
            state.set_flash_message("Aucun changement non stage a stasher");
        }
    }

    Ok(())
}
