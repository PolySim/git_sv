use crate::error::Result;
use crate::git::conflict::ConflictResolutionMode;
use crate::state::{AppState, ConflictPanelFocus, ViewMode};

use super::navigation::{handle_accept_ours_file, handle_accept_theirs_file};
use super::shared::{adjust_scroll, calculate_absolute_line_position};

pub(super) fn handle_set_mode_file(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.resolution_mode = ConflictResolutionMode::File;
        conflicts.line_selected = 0;
        conflicts.result_scroll = 0;
    }
    Ok(())
}

pub(super) fn handle_set_mode_block(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::ConflictType;

    if let Some(ref mut conflicts) = state.conflicts_state {
        // Vérifier si le fichier courant est un conflit de suppression
        let is_deletion_conflict = conflicts
            .all_files
            .get(conflicts.file_selected)
            .map(|f| {
                matches!(
                    f.conflict_type,
                    Some(ConflictType::DeletedByUs | ConflictType::DeletedByThem)
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

pub(super) fn handle_set_mode_line(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::ConflictType;

    if let Some(ref mut conflicts) = state.conflicts_state {
        // Vérifier si le fichier courant est un conflit de suppression
        let is_deletion_conflict = conflicts
            .all_files
            .get(conflicts.file_selected)
            .map(|f| {
                matches!(
                    f.conflict_type,
                    Some(ConflictType::DeletedByUs | ConflictType::DeletedByThem)
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

pub(super) fn handle_toggle_line(state: &mut AppState) -> Result<()> {
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

pub(super) fn handle_line_down(state: &mut AppState) -> Result<()> {
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

        let is_file_mode = conflicts.resolution_mode == ConflictResolutionMode::File;
        let file_selected = conflicts.file_selected;
        let _section_selected_before = conflicts.section_selected;

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

        // Calculer la position absolue pour le scroll
        let absolute_line = conflicts.all_files.get(file_selected).map(|file| {
            calculate_absolute_line_position(
                file,
                conflicts.section_selected,
                conflicts.line_selected,
                is_file_mode,
            )
            .0
        });

        // Mettre à jour le scroll pour garder la ligne visible
        if let Some(line) = absolute_line {
            // On doit sortir du scope du borrow mutable avant d'appeler update_scrolls_for_position
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
    Ok(())
}

pub(super) fn handle_line_up(state: &mut AppState) -> Result<()> {
    use crate::state::ConflictPanelFocus;

    if let Some(ref mut conflicts) = state.conflicts_state {
        let is_file_mode = conflicts.resolution_mode == ConflictResolutionMode::File;
        let file_selected = conflicts.file_selected;

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

        // Calculer la position absolue pour le scroll
        let absolute_line = conflicts.all_files.get(file_selected).map(|file| {
            calculate_absolute_line_position(
                file,
                conflicts.section_selected,
                conflicts.line_selected,
                is_file_mode,
            )
            .0
        });

        // Mettre à jour le scroll pour garder la ligne visible
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
    Ok(())
}

pub(super) fn handle_result_scroll_down(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.result_scroll += 1;
    }
    Ok(())
}

pub(super) fn handle_result_scroll_up(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.result_scroll > 0 {
            conflicts.result_scroll -= 1;
        }
    }
    Ok(())
}

pub(super) fn handle_start_editing(state: &mut AppState) -> Result<()> {
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

pub(super) fn handle_stop_editing(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.is_editing = false;
    }
    Ok(())
}

pub(super) fn handle_edit_insert_char(state: &mut AppState, c: char) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let line_idx = conflicts.edit_cursor_line;
        let col_idx = conflicts.edit_cursor_col;

        if let Some(line) = conflicts.edit_buffer.get_mut(line_idx) {
            // Insérer le caractère à la position du curseur
            if col_idx <= line.chars().count() {
                line.insert(char_to_byte_index(line, col_idx), c);
                conflicts.edit_cursor_col += 1;
            }
        }
    }
    Ok(())
}

pub(super) fn handle_edit_backspace(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let line_idx = conflicts.edit_cursor_line;
        let col_idx = conflicts.edit_cursor_col;

        if col_idx > 0 {
            // Supprimer le caractère avant le curseur
            if let Some(line) = conflicts.edit_buffer.get_mut(line_idx) {
                if col_idx <= line.chars().count() {
                    line.remove(char_to_byte_index(line, col_idx - 1));
                    conflicts.edit_cursor_col -= 1;
                }
            }
        } else if line_idx > 0 {
            // Fusionner avec la ligne précédente
            let current_line = conflicts.edit_buffer.remove(line_idx);
            conflicts.edit_cursor_line -= 1;
            if let Some(prev_line) = conflicts.edit_buffer.get_mut(conflicts.edit_cursor_line) {
                conflicts.edit_cursor_col = prev_line.chars().count();
                prev_line.push_str(&current_line);
            }
        }
    }
    Ok(())
}

pub(super) fn handle_edit_delete(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let line_idx = conflicts.edit_cursor_line;
        let col_idx = conflicts.edit_cursor_col;
        let buffer_len = conflicts.edit_buffer.len();

        // Vérifier d'abord si on doit supprimer un caractère ou fusionner
        let should_merge = if let Some(line) = conflicts.edit_buffer.get(line_idx) {
            col_idx >= line.chars().count() && line_idx + 1 < buffer_len
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
            if col_idx < line.chars().count() {
                // Supprimer le caractère sous le curseur
                line.remove(char_to_byte_index(line, col_idx));
            }
        }
    }
    Ok(())
}

pub(super) fn handle_edit_cursor_up(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.edit_cursor_line > 0 {
            conflicts.edit_cursor_line -= 1;
            // Ajuster la colonne si la ligne précédente est plus courte
            if let Some(line) = conflicts.edit_buffer.get(conflicts.edit_cursor_line) {
                let line_len = line.chars().count();
                if conflicts.edit_cursor_col > line_len {
                    conflicts.edit_cursor_col = line_len;
                }
            }
        }
    }
    Ok(())
}

pub(super) fn handle_edit_cursor_down(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.edit_cursor_line + 1 < conflicts.edit_buffer.len() {
            conflicts.edit_cursor_line += 1;
            // Ajuster la colonne si la ligne suivante est plus courte
            if let Some(line) = conflicts.edit_buffer.get(conflicts.edit_cursor_line) {
                let line_len = line.chars().count();
                if conflicts.edit_cursor_col > line_len {
                    conflicts.edit_cursor_col = line_len;
                }
            }
        }
    }
    Ok(())
}

pub(super) fn handle_edit_cursor_left(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.edit_cursor_col > 0 {
            conflicts.edit_cursor_col -= 1;
        } else if conflicts.edit_cursor_line > 0 {
            // Aller à la fin de la ligne précédente
            conflicts.edit_cursor_line -= 1;
            if let Some(line) = conflicts.edit_buffer.get(conflicts.edit_cursor_line) {
                conflicts.edit_cursor_col = line.chars().count();
            }
        }
    }
    Ok(())
}

pub(super) fn handle_edit_cursor_right(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if let Some(line) = conflicts.edit_buffer.get(conflicts.edit_cursor_line) {
            if conflicts.edit_cursor_col < line.chars().count() {
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

pub(super) fn handle_edit_newline(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        let line_idx = conflicts.edit_cursor_line;
        let col_idx = conflicts.edit_cursor_col;

        if let Some(line) = conflicts.edit_buffer.get_mut(line_idx) {
            // Splitter la ligne en deux
            let new_line = line.split_off(char_to_byte_index(line, col_idx));
            conflicts.edit_buffer.insert(line_idx + 1, new_line);
            conflicts.edit_cursor_line += 1;
            conflicts.edit_cursor_col = 0;
        }
    }
    Ok(())
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

pub(super) fn handle_leave_view(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Conflicts {
        state.enter_view(ViewMode::Staging);
    }
    Ok(())
}

pub(super) fn handle_enter_resolve(state: &mut AppState) -> Result<()> {
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
