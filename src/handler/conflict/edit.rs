use crate::error::Result;
use crate::state::AppState;
use crate::utils::{flash_error, flash_success};

pub(super) fn handle_start_edit(state: &mut AppState) -> Result<()> {
    super::modes::handle_start_editing(state)
}

pub(super) fn handle_confirm_edit(state: &mut AppState) -> Result<()> {
    let Some(conflicts) = state
        .conflicts_state
        .as_ref()
        .filter(|conflicts| conflicts.is_editing)
    else {
        return Ok(());
    };
    let Some(file) = conflicts.all_files.get(conflicts.file_selected) else {
        return Ok(());
    };
    let file_path = file.path.clone();
    let Some((original_path, original)) = &conflicts.edit_original else {
        return Ok(());
    };
    let absolute_path = match crate::git::conflict::worktree_path(&state.repo.repo, &file_path) {
        Ok(path) => path,
        Err(error) => {
            state.set_flash_message(flash_error("chemin du fichier", error));
            return Ok(());
        }
    };
    // Refuser une écriture périmée : conserver le buffer pour permettre sa récupération.
    if original_path != &file_path
        || std::fs::read(&absolute_path).as_deref().ok() != Some(original.as_bytes())
    {
        state.set_flash_message("Le fichier a changé depuis le début de l'édition. Sauvegarde annulée ; le buffer est conservé.");
        return Ok(());
    }
    let content = conflicts.edit_buffer.join("\n");
    let write_result = (|| -> Result<()> {
        std::fs::write(&absolute_path, content.as_bytes())?;
        crate::git::conflict::stage_resolution(
            &state.repo.repo,
            std::path::Path::new(&file_path),
            false,
        )?;
        Ok(())
    })();
    if let Err(error) = write_result {
        // Si seul l'index a échoué, autoriser une nouvelle tentative avec le même buffer.
        if std::fs::read(&absolute_path).as_deref().ok() == Some(content.as_bytes()) {
            if let Some(conflicts) = &mut state.conflicts_state {
                conflicts.edit_original = Some((file_path.clone(), content));
            }
        }
        state.set_flash_message(flash_error("sauvegarde du conflit", error));
        return Ok(());
    }
    if let Some(conflicts) = &mut state.conflicts_state {
        if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
            file.is_resolved = true;
            file.resolved_content = Some(std::mem::take(&mut conflicts.edit_buffer));
        }
    }
    super::modes::handle_stop_editing(state)?;
    state.mark_dirty();
    state.set_flash_message(flash_success(format!(
        "{} sauvegardé et marqué comme résolu",
        file_path
    )));
    Ok(())
}

pub(super) fn handle_cancel_edit(state: &mut AppState) -> Result<()> {
    super::modes::handle_stop_editing(state)
}
