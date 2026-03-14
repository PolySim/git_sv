use crate::error::Result;
use crate::git::conflict::ConflictResolutionMode;
use crate::state::{AppState, ConflictPanelFocus, ViewMode};

use super::shared::{adjust_scroll, advance_to_next_unresolved, calculate_absolute_line_position};

pub(super) fn handle_previous_file(state: &mut AppState) -> Result<()> {
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

pub(super) fn handle_next_file(state: &mut AppState) -> Result<()> {
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

pub(super) fn handle_previous_section(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let is_file_mode = conflicts.resolution_mode == ConflictResolutionMode::File;
        let file_selected = conflicts.file_selected;

        if conflicts.section_selected > 0 {
            conflicts.section_selected -= 1;

            // Calculer la position absolue pour le scroll (début de la section)
            let absolute_line = conflicts.all_files.get(file_selected).map(|file| {
                calculate_absolute_line_position(
                    file,
                    conflicts.section_selected,
                    0, // Début de la section
                    is_file_mode,
                )
                .0
            });

            // Mettre à jour le scroll pour positionner la section au début de la vue
            if let Some(line) = absolute_line {
                let panel_focus = conflicts.panel_focus;
                let visible_height = match panel_focus {
                    ConflictPanelFocus::OursPanel => conflicts.ours_panel_height,
                    ConflictPanelFocus::TheirsPanel => conflicts.theirs_panel_height,
                    _ => 0,
                };
                let scroll_ref = match panel_focus {
                    ConflictPanelFocus::OursPanel => &mut conflicts.ours_scroll,
                    ConflictPanelFocus::TheirsPanel => &mut conflicts.theirs_scroll,
                    _ => return Ok(()),
                };

                if visible_height > 0 {
                    *scroll_ref = adjust_scroll(line, *scroll_ref, visible_height);
                }
            }
        }
    }
    Ok(())
}

pub(super) fn handle_next_section(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let is_file_mode = conflicts.resolution_mode == ConflictResolutionMode::File;
        let file_selected = conflicts.file_selected;
        let file = &conflicts.all_files[conflicts.file_selected];
        let max_section = file.conflicts.len().saturating_sub(1);

        if conflicts.section_selected < max_section {
            conflicts.section_selected += 1;
            conflicts.line_selected = 0;

            // Calculer la position absolue pour le scroll (début de la section)
            let absolute_line = conflicts.all_files.get(file_selected).map(|file| {
                calculate_absolute_line_position(
                    file,
                    conflicts.section_selected,
                    0, // Début de la section
                    is_file_mode,
                )
                .0
            });

            // Mettre à jour le scroll pour positionner la section au début de la vue
            if let Some(line) = absolute_line {
                let panel_focus = conflicts.panel_focus;
                let visible_height = match panel_focus {
                    ConflictPanelFocus::OursPanel => conflicts.ours_panel_height,
                    ConflictPanelFocus::TheirsPanel => conflicts.theirs_panel_height,
                    _ => 0,
                };
                let scroll_ref = match panel_focus {
                    ConflictPanelFocus::OursPanel => &mut conflicts.ours_scroll,
                    ConflictPanelFocus::TheirsPanel => &mut conflicts.theirs_scroll,
                    _ => return Ok(()),
                };

                if visible_height > 0 {
                    *scroll_ref = adjust_scroll(line, *scroll_ref, visible_height);
                }
            }
        }
    }
    Ok(())
}

pub(super) fn handle_switch_panel(state: &mut AppState) -> Result<()> {
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

pub(super) fn handle_accept_ours_file(state: &mut AppState) -> Result<()> {
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
                    Some(ConflictType::DeletedByUs | ConflictType::DeletedByThem)
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

pub(super) fn handle_accept_theirs_file(state: &mut AppState) -> Result<()> {
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
                    Some(ConflictType::DeletedByUs | ConflictType::DeletedByThem)
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

pub(super) fn handle_accept_ours_block(state: &mut AppState) -> Result<()> {
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

pub(super) fn handle_accept_theirs_block(state: &mut AppState) -> Result<()> {
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

pub(super) fn handle_accept_both(state: &mut AppState) -> Result<()> {
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

pub(super) fn handle_mark_resolved(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::{all_sections_resolved, apply_resolved_content};

    let (file_path, all_resolved) = if let Some(conflicts) = &state.conflicts_state {
        let path = conflicts
            .all_files
            .get(conflicts.file_selected)
            .map(|f| f.path.clone());
        let resolved = conflicts
            .all_files
            .get(conflicts.file_selected)
            .map(all_sections_resolved)
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

pub(super) fn handle_finalize_merge(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Conflicts {
        match crate::git::conflict::finalize_merge(&state.repo.repo, "Merge finalisé") {
            Ok(_) => {
                state.clear_conflicts();
                state.enter_view(ViewMode::Graph);
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

pub(super) fn handle_abort_merge(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Conflicts {
        match crate::git::conflict::abort_merge(&state.repo.repo) {
            Ok(_) => {
                state.clear_conflicts();
                state.enter_view(ViewMode::Staging);
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
