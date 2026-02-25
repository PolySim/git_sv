//! Handler pour les actions de résolution de conflits.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::git::conflict::ConflictResolutionMode;
use crate::state::action::ConflictAction;
use crate::state::{AppState, ConflictPanelFocus, ViewMode};

/// Handler pour les opérations de résolution de conflits.
pub struct ConflictHandler;

impl ActionHandler for ConflictHandler {
    type Action = ConflictAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: ConflictAction) -> Result<()> {
        match action {
            ConflictAction::PreviousFile => handle_previous_file(ctx.state),
            ConflictAction::NextFile => handle_next_file(ctx.state),
            ConflictAction::PreviousSection => handle_previous_section(ctx.state),
            ConflictAction::NextSection => handle_next_section(ctx.state),
            ConflictAction::SwitchPanel => handle_switch_panel(ctx.state),
            ConflictAction::AcceptOursFile => handle_accept_ours_file(ctx.state),
            ConflictAction::AcceptTheirsFile => handle_accept_theirs_file(ctx.state),
            ConflictAction::AcceptOursBlock => handle_accept_ours_block(ctx.state),
            ConflictAction::AcceptTheirsBlock => handle_accept_theirs_block(ctx.state),
            ConflictAction::AcceptBoth => handle_accept_both(ctx.state),
            ConflictAction::StartEdit => handle_start_edit(ctx.state),
            ConflictAction::ConfirmEdit => handle_confirm_edit(ctx.state),
            ConflictAction::CancelEdit => handle_cancel_edit(ctx.state),
            ConflictAction::MarkResolved => handle_mark_resolved(ctx.state),
            ConflictAction::FinalizeMerge => handle_finalize_merge(ctx.state),
            ConflictAction::AbortMerge => handle_abort_merge(ctx.state),
            ConflictAction::SetModeFile => handle_set_mode_file(ctx.state),
            ConflictAction::SetModeBlock => handle_set_mode_block(ctx.state),
            ConflictAction::SetModeLine => handle_set_mode_line(ctx.state),
            ConflictAction::ToggleLine => handle_toggle_line(ctx.state),
            ConflictAction::LineDown => handle_line_down(ctx.state),
            ConflictAction::LineUp => handle_line_up(ctx.state),
            ConflictAction::ResultScrollDown => handle_result_scroll_down(ctx.state),
            ConflictAction::ResultScrollUp => handle_result_scroll_up(ctx.state),
            ConflictAction::StartEditing => handle_start_editing(ctx.state),
            ConflictAction::StopEditing => handle_stop_editing(ctx.state),
            ConflictAction::EditInsertChar(c) => handle_edit_insert_char(ctx.state, c),
            ConflictAction::EditBackspace => handle_edit_backspace(ctx.state),
            ConflictAction::EditDelete => handle_edit_delete(ctx.state),
            ConflictAction::EditCursorUp => handle_edit_cursor_up(ctx.state),
            ConflictAction::EditCursorDown => handle_edit_cursor_down(ctx.state),
            ConflictAction::EditCursorLeft => handle_edit_cursor_left(ctx.state),
            ConflictAction::EditCursorRight => handle_edit_cursor_right(ctx.state),
            ConflictAction::EditNewline => handle_edit_newline(ctx.state),
            ConflictAction::LeaveView => handle_leave_view(ctx.state),
            ConflictAction::EnterResolve => handle_enter_resolve(ctx.state),
        }
    }
}

fn handle_previous_file(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.file_selected > 0 {
            conflicts.file_selected -= 1;
            conflicts.section_selected = 0;
            conflicts.line_selected = 0;
            conflicts.result_scroll = 0;
            conflicts.ours_scroll = 0;
            conflicts.theirs_scroll = 0;
        }
    }
    Ok(())
}

fn handle_next_file(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let file_count = conflicts.all_files.len();
        if conflicts.file_selected + 1 < file_count {
            conflicts.file_selected += 1;
            conflicts.section_selected = 0;
            conflicts.line_selected = 0;
            conflicts.result_scroll = 0;
            conflicts.ours_scroll = 0;
            conflicts.theirs_scroll = 0;
        }
    }
    Ok(())
}

fn handle_previous_section(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.section_selected > 0 {
            conflicts.section_selected -= 1;
        }
    }
    Ok(())
}

fn handle_next_section(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let file = &conflicts.all_files[conflicts.file_selected];
        let max_section = file.conflicts.len().saturating_sub(1);
        if conflicts.section_selected < max_section {
            conflicts.section_selected += 1;
            conflicts.line_selected = 0; // Reset la sélection de ligne
        }
    }
    Ok(())
}

fn handle_switch_panel(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.panel_focus = match conflicts.panel_focus {
            ConflictPanelFocus::FileList => ConflictPanelFocus::OursPanel,
            ConflictPanelFocus::OursPanel => ConflictPanelFocus::TheirsPanel,
            ConflictPanelFocus::TheirsPanel => ConflictPanelFocus::ResultPanel,
            ConflictPanelFocus::ResultPanel => ConflictPanelFocus::FileList,
        };
    }
    Ok(())
}

fn handle_accept_ours_file(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::{
        resolve_file_with_strategy, resolve_special_file, ConflictResolution, ConflictType,
    };

    let (file_path, is_special) = state
        .conflicts_state
        .as_ref()
        .and_then(|c| c.all_files.get(c.file_selected))
        .map(|f| {
            (
                f.path.clone(),
                matches!(
                    f.conflict_type,
                    Some(ConflictType::DeletedByUs) | Some(ConflictType::DeletedByThem)
                ),
            )
        })
        .unzip();

    let file_path = match file_path {
        Some(p) => p,
        None => return Ok(()),
    };
    let is_special = is_special.unwrap_or(false);
    let file_index = state
        .conflicts_state
        .as_ref()
        .map(|c| c.file_selected)
        .unwrap_or(0);

    let result = if is_special {
        // Pour les conflits de suppression, utiliser resolve_special_file
        if let Some(ref conflicts) = state.conflicts_state {
            if let Some(file) = conflicts.all_files.get(file_index) {
                resolve_special_file(&state.repo.repo, file, ConflictResolution::Ours)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    } else {
        // Pour les conflits classiques, utiliser resolve_file_with_strategy
        resolve_file_with_strategy(&state.repo.repo, &file_path, ConflictResolution::Ours)
            .map(|_| false)
    };

    match result {
        Ok(_) => {
            // Mettre à jour l'état en mémoire
            if let Some(ref mut conflicts) = state.conflicts_state {
                if let Some(file) = conflicts.all_files.get_mut(file_index) {
                    file.is_resolved = true;
                    for conflict in &mut file.conflicts {
                        conflict.resolution = Some(ConflictResolution::Ours);
                    }
                }
                // Avancer au fichier suivant non résolu
                advance_to_next_unresolved(conflicts);
            }
            state.mark_dirty();
            state.set_flash_message(format!("Accepté 'ours' pour {}", file_path));
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur: {}", e));
        }
    }

    Ok(())
}

fn handle_accept_theirs_file(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::{
        resolve_file_with_strategy, resolve_special_file, ConflictResolution, ConflictType,
    };

    let (file_path, is_special) = state
        .conflicts_state
        .as_ref()
        .and_then(|c| c.all_files.get(c.file_selected))
        .map(|f| {
            (
                f.path.clone(),
                matches!(
                    f.conflict_type,
                    Some(ConflictType::DeletedByUs) | Some(ConflictType::DeletedByThem)
                ),
            )
        })
        .unzip();

    let file_path = match file_path {
        Some(p) => p,
        None => return Ok(()),
    };
    let is_special = is_special.unwrap_or(false);
    let file_index = state
        .conflicts_state
        .as_ref()
        .map(|c| c.file_selected)
        .unwrap_or(0);

    let result = if is_special {
        // Pour les conflits de suppression, utiliser resolve_special_file
        if let Some(ref conflicts) = state.conflicts_state {
            if let Some(file) = conflicts.all_files.get(file_index) {
                resolve_special_file(&state.repo.repo, file, ConflictResolution::Theirs)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    } else {
        // Pour les conflits classiques, utiliser resolve_file_with_strategy
        resolve_file_with_strategy(&state.repo.repo, &file_path, ConflictResolution::Theirs)
            .map(|_| false)
    };

    match result {
        Ok(_) => {
            // Mettre à jour l'état en mémoire
            if let Some(ref mut conflicts) = state.conflicts_state {
                if let Some(file) = conflicts.all_files.get_mut(file_index) {
                    file.is_resolved = true;
                    for conflict in &mut file.conflicts {
                        conflict.resolution = Some(ConflictResolution::Theirs);
                    }
                }
                // Avancer au fichier suivant non résolu
                advance_to_next_unresolved(conflicts);
            }
            state.mark_dirty();
            state.set_flash_message(format!("Accepté 'theirs' pour {}", file_path));
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur: {}", e));
        }
    }

    Ok(())
}

fn handle_accept_ours_block(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::{all_sections_resolved, apply_resolved_content, ConflictResolution};

    let should_apply = if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        let file_selected = conflicts.file_selected;

        if let Some(file) = conflicts.all_files.get_mut(file_selected) {
            if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                conflict.resolution = Some(ConflictResolution::Ours);
            }

            // Vérifier si toutes les sections sont résolues
            all_sections_resolved(file)
        } else {
            false
        }
    } else {
        false
    };

    if should_apply {
        // Appliquer la résolution sur le disque
        let (file_path, mode) = if let Some(conflicts) = &state.conflicts_state {
            let file = &conflicts.all_files[conflicts.file_selected];
            (file.path.clone(), conflicts.resolution_mode)
        } else {
            return Ok(());
        };

        if let Some(conflicts) = &mut state.conflicts_state {
            if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
                if let Err(e) = apply_resolved_content(&state.repo.repo, file, mode) {
                    state.set_flash_message(format!(
                        "Erreur lors de l'application de la résolution: {}",
                        e
                    ));
                    return Ok(());
                }

                file.is_resolved = true;
            }
        }

        state.set_flash_message(format!("{} résolu (ours)", file_path));

        if let Some(conflicts) = &mut state.conflicts_state {
            advance_to_next_unresolved(conflicts);
        }
        state.mark_dirty();
    }

    Ok(())
}

fn handle_accept_theirs_block(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::{all_sections_resolved, apply_resolved_content, ConflictResolution};

    let should_resolve = if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        let file_selected = conflicts.file_selected;

        if let Some(file) = conflicts.all_files.get_mut(file_selected) {
            if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                conflict.resolution = Some(ConflictResolution::Theirs);
            }
            all_sections_resolved(file)
        } else {
            false
        }
    } else {
        false
    };

    if should_resolve {
        let (path, mode) = if let Some(conflicts) = &state.conflicts_state {
            let file = &conflicts.all_files[conflicts.file_selected];
            (file.path.clone(), conflicts.resolution_mode)
        } else {
            return Ok(());
        };

        if let Some(conflicts) = &mut state.conflicts_state {
            if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
                if let Err(e) = apply_resolved_content(&state.repo.repo, file, mode) {
                    state.set_flash_message(format!("Erreur: {}", e));
                    return Ok(());
                }
                file.is_resolved = true;
            }
            advance_to_next_unresolved(conflicts);
        }
        state.set_flash_message(format!("{} résolu (theirs)", path));
        state.mark_dirty();
    }
    Ok(())
}

fn handle_accept_both(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::{all_sections_resolved, apply_resolved_content, ConflictResolution};

    let should_resolve = if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        let file_selected = conflicts.file_selected;

        if let Some(file) = conflicts.all_files.get_mut(file_selected) {
            if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                conflict.resolution = Some(ConflictResolution::Both);
            }
            all_sections_resolved(file)
        } else {
            false
        }
    } else {
        false
    };

    if should_resolve {
        let (path, mode) = if let Some(conflicts) = &state.conflicts_state {
            let file = &conflicts.all_files[conflicts.file_selected];
            (file.path.clone(), conflicts.resolution_mode)
        } else {
            return Ok(());
        };

        if let Some(conflicts) = &mut state.conflicts_state {
            if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
                if let Err(e) = apply_resolved_content(&state.repo.repo, file, mode) {
                    state.set_flash_message(format!("Erreur: {}", e));
                    return Ok(());
                }
                file.is_resolved = true;
            }
            advance_to_next_unresolved(conflicts);
        }
        state.set_flash_message(format!("{} résolu (both)", path));
        state.mark_dirty();
    }
    Ok(())
}

fn handle_start_edit(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.is_editing = true;
    }
    Ok(())
}

fn handle_confirm_edit(state: &mut AppState) -> Result<()> {
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

fn handle_cancel_edit(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.is_editing = false;
    }
    Ok(())
}

fn handle_mark_resolved(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::{all_sections_resolved, apply_resolved_content};

    let (file_path, all_resolved) = if let Some(conflicts) = &state.conflicts_state {
        let path = conflicts
            .all_files
            .get(conflicts.file_selected)
            .map(|f| f.path.clone());
        let resolved = conflicts
            .all_files
            .get(conflicts.file_selected)
            .map(|f| all_sections_resolved(f))
            .unwrap_or(false);
        (path, resolved)
    } else {
        (None, false)
    };

    if let Some(path) = file_path {
        if all_resolved {
            let mode = if let Some(conflicts) = &state.conflicts_state {
                conflicts.resolution_mode
            } else {
                return Ok(());
            };

            if let Some(conflicts) = &mut state.conflicts_state {
                if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
                    // Appliquer la résolution sur le disque
                    if let Err(e) = apply_resolved_content(&state.repo.repo, file, mode) {
                        state.set_flash_message(format!(
                            "Erreur lors de l'application de la résolution: {}",
                            e
                        ));
                        return Ok(());
                    }

                    file.is_resolved = true;
                }
                advance_to_next_unresolved(conflicts);
            }
            state.set_flash_message(format!("{} résolu et sauvegardé", path));
            state.mark_dirty();
        } else {
            state.set_flash_message(format!(
                "{}: toutes les sections ne sont pas résolues",
                path
            ));
        }
    }
    Ok(())
}

fn handle_finalize_merge(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Conflicts {
        match crate::git::conflict::finalize_merge(&state.repo.repo, "Merge finalisé") {
            Ok(_) => {
                state.conflicts_state = None;
                state.view_mode = ViewMode::Graph;
                state.mark_dirty();
                state.set_flash_message("Merge finalisé ✓".to_string());
            }
            Err(e) => {
                state.set_flash_message(format!("Erreur: {}", e));
            }
        }
    }
    Ok(())
}

fn handle_abort_merge(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Conflicts {
        match crate::git::conflict::abort_merge(&state.repo.repo) {
            Ok(_) => {
                state.conflicts_state = None;
                state.view_mode = ViewMode::Staging;
                state.mark_dirty();
                state.set_flash_message("Merge annulé".to_string());
            }
            Err(e) => {
                state.set_flash_message(format!("Erreur: {}", e));
            }
        }
    }
    Ok(())
}

fn handle_set_mode_file(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.resolution_mode = ConflictResolutionMode::File;
        conflicts.line_selected = 0;
        conflicts.result_scroll = 0;
    }
    Ok(())
}

fn handle_set_mode_block(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::ConflictType;

    if let Some(ref mut conflicts) = state.conflicts_state {
        // Vérifier si le fichier courant est un conflit de suppression
        let is_deletion_conflict = conflicts
            .all_files
            .get(conflicts.file_selected)
            .map(|f| {
                matches!(
                    f.conflict_type,
                    Some(ConflictType::DeletedByUs) | Some(ConflictType::DeletedByThem)
                )
            })
            .unwrap_or(false);

        if is_deletion_conflict {
            state.set_flash_message(
                "Mode bloc non disponible pour les conflits de suppression".to_string(),
            );
            return Ok(());
        }

        conflicts.resolution_mode = ConflictResolutionMode::Block;
        conflicts.line_selected = 0;
        conflicts.result_scroll = 0;
    }
    Ok(())
}

fn handle_set_mode_line(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::ConflictType;

    if let Some(ref mut conflicts) = state.conflicts_state {
        // Vérifier si le fichier courant est un conflit de suppression
        let is_deletion_conflict = conflicts
            .all_files
            .get(conflicts.file_selected)
            .map(|f| {
                matches!(
                    f.conflict_type,
                    Some(ConflictType::DeletedByUs) | Some(ConflictType::DeletedByThem)
                )
            })
            .unwrap_or(false);

        if is_deletion_conflict {
            state.set_flash_message(
                "Mode ligne non disponible pour les conflits de suppression".to_string(),
            );
            return Ok(());
        }

        conflicts.resolution_mode = ConflictResolutionMode::Line;
        conflicts.line_selected = 0;
        conflicts.result_scroll = 0;
    }
    Ok(())
}

fn handle_toggle_line(state: &mut AppState) -> Result<()> {
    use crate::state::ConflictPanelFocus;

    if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        let line_idx = conflicts.line_selected;
        let file_selected = conflicts.file_selected;

        if let Some(file) = conflicts.all_files.get_mut(file_selected) {
            if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                // Assurer que line_level_resolution existe
                if conflict.line_level_resolution.is_none() {
                    conflict.line_level_resolution =
                        Some(crate::git::conflict::LineLevelResolution::new(
                            conflict.ours.len(),
                            conflict.theirs.len(),
                        ));
                }

                match conflicts.panel_focus {
                    ConflictPanelFocus::OursPanel => {
                        if let Some(resolution) = &mut conflict.line_level_resolution {
                            if let Some(included) = resolution.ours_lines_included.get_mut(line_idx)
                            {
                                *included = !*included;
                                resolution.touched = true;
                            }
                        }
                    }
                    ConflictPanelFocus::TheirsPanel => {
                        if let Some(resolution) = &mut conflict.line_level_resolution {
                            if let Some(included) =
                                resolution.theirs_lines_included.get_mut(line_idx)
                            {
                                *included = !*included;
                                resolution.touched = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn handle_line_down(state: &mut AppState) -> Result<()> {
    use crate::state::ConflictPanelFocus;

    if let Some(conflicts) = &mut state.conflicts_state {
        let max_lines = if let Some(file) = conflicts.all_files.get(conflicts.file_selected) {
            if let Some(conflict) = file.conflicts.get(conflicts.section_selected) {
                match conflicts.panel_focus {
                    ConflictPanelFocus::OursPanel => conflict.ours.len(),
                    ConflictPanelFocus::TheirsPanel => conflict.theirs.len(),
                    _ => 0,
                }
            } else {
                0
            }
        } else {
            0
        };

        if conflicts.line_selected < max_lines.saturating_sub(1) {
            // Naviguer dans les lignes du block courant
            conflicts.line_selected += 1;
        } else {
            // En fin de block, passer au block suivant si disponible
            let file = &conflicts.all_files[conflicts.file_selected];
            if conflicts.section_selected + 1 < file.conflicts.len() {
                conflicts.section_selected += 1;
                conflicts.line_selected = 0;
            }
        }
    }
    Ok(())
}

fn handle_line_up(state: &mut AppState) -> Result<()> {
    use crate::state::ConflictPanelFocus;

    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.line_selected > 0 {
            // Naviguer dans les lignes du block courant
            conflicts.line_selected -= 1;
        } else if conflicts.section_selected > 0 {
            // En début de block, passer au block précédent
            conflicts.section_selected -= 1;
            let file = &conflicts.all_files[conflicts.file_selected];
            let prev_section = &file.conflicts[conflicts.section_selected];
            let max_lines = match conflicts.panel_focus {
                ConflictPanelFocus::OursPanel => prev_section.ours.len(),
                ConflictPanelFocus::TheirsPanel => prev_section.theirs.len(),
                _ => 0,
            };
            conflicts.line_selected = max_lines.saturating_sub(1);
        }
    }
    Ok(())
}

fn handle_result_scroll_down(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.result_scroll += 1;
    }
    Ok(())
}

fn handle_result_scroll_up(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.result_scroll > 0 {
            conflicts.result_scroll -= 1;
        }
    }
    Ok(())
}

fn handle_start_editing(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::generate_resolved_content_with_source;

    if let Some(ref mut conflicts) = state.conflicts_state {
        // Générer le contenu résolu actuel en utilisant la fonction qui prend
        // en compte les résolutions ligne par ligne (line_level_resolution)
        if let Some(file) = conflicts.all_files.get(conflicts.file_selected) {
            let mode = conflicts.resolution_mode;
            let resolved = generate_resolved_content_with_source(file, mode);

            // Convertir les ResolvedLine en String
            conflicts.edit_buffer = resolved.into_iter().map(|line| line.content).collect();

            // Positionner le curseur au début
            conflicts.edit_cursor_line = 0;
            conflicts.edit_cursor_col = 0;
        }

        conflicts.is_editing = true;
    }
    Ok(())
}

fn handle_stop_editing(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.is_editing = false;
    }
    Ok(())
}

fn handle_edit_insert_char(state: &mut AppState, c: char) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let line_idx = conflicts.edit_cursor_line;
        let col_idx = conflicts.edit_cursor_col;

        if let Some(line) = conflicts.edit_buffer.get_mut(line_idx) {
            // Insérer le caractère à la position du curseur
            if col_idx <= line.len() {
                line.insert(col_idx, c);
                conflicts.edit_cursor_col += 1;
            }
        }
    }
    Ok(())
}

fn handle_edit_backspace(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let line_idx = conflicts.edit_cursor_line;
        let col_idx = conflicts.edit_cursor_col;

        if col_idx > 0 {
            // Supprimer le caractère avant le curseur
            if let Some(line) = conflicts.edit_buffer.get_mut(line_idx) {
                if col_idx <= line.len() {
                    line.remove(col_idx - 1);
                    conflicts.edit_cursor_col -= 1;
                }
            }
        } else if line_idx > 0 {
            // Fusionner avec la ligne précédente
            let current_line = conflicts.edit_buffer.remove(line_idx);
            conflicts.edit_cursor_line -= 1;
            if let Some(prev_line) = conflicts.edit_buffer.get_mut(conflicts.edit_cursor_line) {
                conflicts.edit_cursor_col = prev_line.len();
                prev_line.push_str(&current_line);
            }
        }
    }
    Ok(())
}

fn handle_edit_delete(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let line_idx = conflicts.edit_cursor_line;
        let col_idx = conflicts.edit_cursor_col;
        let buffer_len = conflicts.edit_buffer.len();

        // Vérifier d'abord si on doit supprimer un caractère ou fusionner
        let should_merge = if let Some(line) = conflicts.edit_buffer.get(line_idx) {
            col_idx >= line.len() && line_idx + 1 < buffer_len
        } else {
            false
        };

        if should_merge {
            // Fusionner avec la ligne suivante
            let next_line = conflicts.edit_buffer.remove(line_idx + 1);
            if let Some(line) = conflicts.edit_buffer.get_mut(line_idx) {
                line.push_str(&next_line);
            }
        } else if let Some(line) = conflicts.edit_buffer.get_mut(line_idx) {
            if col_idx < line.len() {
                // Supprimer le caractère sous le curseur
                line.remove(col_idx);
            }
        }
    }
    Ok(())
}

fn handle_edit_cursor_up(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.edit_cursor_line > 0 {
            conflicts.edit_cursor_line -= 1;
            // Ajuster la colonne si la ligne précédente est plus courte
            if let Some(line) = conflicts.edit_buffer.get(conflicts.edit_cursor_line) {
                if conflicts.edit_cursor_col > line.len() {
                    conflicts.edit_cursor_col = line.len();
                }
            }
        }
    }
    Ok(())
}

fn handle_edit_cursor_down(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.edit_cursor_line + 1 < conflicts.edit_buffer.len() {
            conflicts.edit_cursor_line += 1;
            // Ajuster la colonne si la ligne suivante est plus courte
            if let Some(line) = conflicts.edit_buffer.get(conflicts.edit_cursor_line) {
                if conflicts.edit_cursor_col > line.len() {
                    conflicts.edit_cursor_col = line.len();
                }
            }
        }
    }
    Ok(())
}

fn handle_edit_cursor_left(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.edit_cursor_col > 0 {
            conflicts.edit_cursor_col -= 1;
        } else if conflicts.edit_cursor_line > 0 {
            // Aller à la fin de la ligne précédente
            conflicts.edit_cursor_line -= 1;
            if let Some(line) = conflicts.edit_buffer.get(conflicts.edit_cursor_line) {
                conflicts.edit_cursor_col = line.len();
            }
        }
    }
    Ok(())
}

fn handle_edit_cursor_right(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if let Some(line) = conflicts.edit_buffer.get(conflicts.edit_cursor_line) {
            if conflicts.edit_cursor_col < line.len() {
                conflicts.edit_cursor_col += 1;
            } else if conflicts.edit_cursor_line + 1 < conflicts.edit_buffer.len() {
                // Aller au début de la ligne suivante
                conflicts.edit_cursor_line += 1;
                conflicts.edit_cursor_col = 0;
            }
        }
    }
    Ok(())
}

fn handle_edit_newline(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let line_idx = conflicts.edit_cursor_line;
        let col_idx = conflicts.edit_cursor_col;

        if let Some(line) = conflicts.edit_buffer.get_mut(line_idx) {
            // Splitter la ligne en deux
            let new_line = line.split_off(col_idx);
            conflicts.edit_buffer.insert(line_idx + 1, new_line);
            conflicts.edit_cursor_line += 1;
            conflicts.edit_cursor_col = 0;
        }
    }
    Ok(())
}

fn handle_leave_view(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Conflicts {
        state.view_mode = ViewMode::Staging;
    }
    Ok(())
}

fn handle_enter_resolve(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::ConflictResolutionMode;
    use crate::state::ConflictPanelFocus;

    if let Some(conflicts) = &mut state.conflicts_state {
        match conflicts.resolution_mode {
            ConflictResolutionMode::File => match conflicts.panel_focus {
                ConflictPanelFocus::OursPanel => handle_accept_ours_file(state)?,
                ConflictPanelFocus::TheirsPanel => handle_accept_theirs_file(state)?,
                _ => {}
            },
            ConflictResolutionMode::Block => {
                use crate::git::conflict::ConflictResolution;

                let section_idx = conflicts.section_selected;
                if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
                    if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                        match conflicts.panel_focus {
                            ConflictPanelFocus::OursPanel => {
                                if conflict.resolution == Some(ConflictResolution::Ours) {
                                    conflict.resolution = None; // Désélectionner
                                } else {
                                    conflict.resolution = Some(ConflictResolution::Ours);
                                }
                            }
                            ConflictPanelFocus::TheirsPanel => {
                                if conflict.resolution == Some(ConflictResolution::Theirs) {
                                    conflict.resolution = None; // Désélectionner
                                } else {
                                    conflict.resolution = Some(ConflictResolution::Theirs);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            ConflictResolutionMode::Line => {
                // En mode Line, Enter est déjà mappé à ConflictResolveFile
                // Ce handler ne fait rien car la validation est gérée par handle_mark_resolved
            }
        }
    }
    Ok(())
}

/// Avance à la sélection au prochain fichier non résolu.
fn advance_to_next_unresolved(conflicts: &mut crate::state::ConflictsState) {
    let current = conflicts.file_selected;
    let total = conflicts.all_files.len();

    // Chercher un fichier non résolu après le courant
    for i in (current + 1)..total {
        if let Some(file) = conflicts.all_files.get(i) {
            if !file.is_resolved {
                conflicts.file_selected = i;
                conflicts.section_selected = 0;
                return;
            }
        }
    }

    // Si aucun trouvé après, chercher depuis le début
    for i in 0..current {
        if let Some(file) = conflicts.all_files.get(i) {
            if !file.is_resolved {
                conflicts.file_selected = i;
                conflicts.section_selected = 0;
                return;
            }
        }
    }

    // Si tous les fichiers sont résolus, rester sur le courant
}
