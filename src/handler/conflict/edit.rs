use crate::error::Result;
use crate::state::AppState;

pub(super) fn handle_start_edit(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.is_editing = true;
    }
    Ok(())
}

pub(super) fn handle_confirm_edit(state: &mut AppState) -> Result<()> {
    use std::io::Write;

    let (file_path, content) = if let Some(ref conflicts) = state.conflicts_state {
        if let Some(file) = conflicts.all_files.get(conflicts.file_selected) {
            let path = file.path.clone();
            let buf_content = conflicts.edit_buffer.join("\n");
            (Some(path), Some(buf_content))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let (file_path, content) = match (file_path, content) {
        (Some(p), Some(c)) => (p, c),
        _ => return Ok(()),
    };

    // Écrire le contenu du buffer dans le fichier
    match std::fs::File::create(&file_path) {
        Ok(mut file_handle) => {
            if let Err(e) = file_handle.write_all(content.as_bytes()) {
                state.set_flash_message(format!("Erreur écriture fichier: {}", e));
                return Ok(());
            }

            // Mettre à jour l'index git
            match state.repo.repo.index() {
                Ok(mut index) => {
                    // Supprimer les entrées existantes pour ce chemin
                    index.remove_path(std::path::Path::new(&file_path)).ok();

                    // Ajouter le fichier résolu à l'index
                    if let Err(e) = index.add_path(std::path::Path::new(&file_path)) {
                        state.set_flash_message(format!("Erreur git add: {}", e));
                        return Ok(());
                    }

                    if let Err(e) = index.write() {
                        state.set_flash_message(format!("Erreur écriture index: {}", e));
                        return Ok(());
                    }
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur accès index: {}", e));
                    return Ok(());
                }
            }

            // Marquer le fichier comme résolu dans l'état
            if let Some(ref mut conflicts) = state.conflicts_state {
                if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
                    file.is_resolved = true;
                }
            }

            state.mark_dirty();
            state.set_flash_message(format!("{} sauvegardé et marqué comme résolu", file_path));
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur création fichier: {}", e));
        }
    }

    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.is_editing = false;
    }

    Ok(())
}

pub(super) fn handle_cancel_edit(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.is_editing = false;
    }
    Ok(())
}
