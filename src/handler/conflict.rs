//! Handler pour les actions de résolution de conflits.

use crate::error::Result;
use crate::state::{AppState, ViewMode, ConflictPanelFocus};
use crate::state::action::ConflictAction;
use crate::git::conflict::ConflictResolutionMode;
use super::traits::{ActionHandler, HandlerContext};

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
    use crate::git::conflict::{ConflictResolution, resolve_file_with_strategy};

    let file_path = state.conflicts_state.as_ref()
        .and_then(|c| c.all_files.get(c.file_selected))
        .map(|f| f.path.clone());

    if let (Some(path), Some(file_index)) = (file_path, state.conflicts_state.as_ref().map(|c| c.file_selected)) {
        if let Err(e) = resolve_file_with_strategy(&state.repo.repo, &path, ConflictResolution::Ours) {
            state.set_flash_message(format!("Erreur: {}", e));
        } else {
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
            state.set_flash_message(format!("Accepté 'ours' pour {}", path));
        }
    }
    Ok(())
}

fn handle_accept_theirs_file(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::{ConflictResolution, resolve_file_with_strategy};

    let file_path = state.conflicts_state.as_ref()
        .and_then(|c| c.all_files.get(c.file_selected))
        .map(|f| f.path.clone());

    if let (Some(path), Some(file_index)) = (file_path, state.conflicts_state.as_ref().map(|c| c.file_selected)) {
        if let Err(e) = resolve_file_with_strategy(&state.repo.repo, &path, ConflictResolution::Theirs) {
            state.set_flash_message(format!("Erreur: {}", e));
        } else {
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
            state.set_flash_message(format!("Accepté 'theirs' pour {}", path));
        }
    }
    Ok(())
}

fn handle_accept_ours_block(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::ConflictResolution;

    if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
            if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                conflict.resolution = Some(ConflictResolution::Ours);
            }
        }
    }
    Ok(())
}

fn handle_accept_theirs_block(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::ConflictResolution;

    if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
            if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                conflict.resolution = Some(ConflictResolution::Theirs);
            }
        }
    }
    Ok(())
}

fn handle_accept_both(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::ConflictResolution;

    if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
            if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                conflict.resolution = Some(ConflictResolution::Both);
            }
        }
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
    let file_path = state.conflicts_state.as_ref()
        .and_then(|c| c.all_files.get(c.file_selected))
        .map(|f| f.path.clone());

    if let (Some(ref mut conflicts), Some(path)) = (state.conflicts_state.as_mut(), file_path) {
        if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
            file.is_resolved = true;
        }
        state.set_flash_message(format!("{} marqué comme résolu", path));
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
    }
    Ok(())
}

fn handle_set_mode_block(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.resolution_mode = ConflictResolutionMode::Block;
        conflicts.line_selected = 0;
    }
    Ok(())
}

fn handle_set_mode_line(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        conflicts.resolution_mode = ConflictResolutionMode::Line;
        conflicts.line_selected = 0;
    }
    Ok(())
}

fn handle_toggle_line(state: &mut AppState) -> Result<()> {
    use crate::state::ConflictPanelFocus;

    if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        let line_idx = conflicts.line_selected;

        if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
            if let Some(conflict) = file.conflicts.get_mut(section_idx) {
                // Assurer que line_level_resolution existe
                if conflict.line_level_resolution.is_none() {
                    conflict.line_level_resolution = Some(crate::git::conflict::LineLevelResolution::new(
                        conflict.ours.len(),
                        conflict.theirs.len(),
                    ));
                }

                match conflicts.panel_focus {
                    ConflictPanelFocus::OursPanel => {
                        if let Some(resolution) = &mut conflict.line_level_resolution {
                            if let Some(included) = resolution.ours_lines_included.get_mut(line_idx) {
                                *included = !*included;
                                resolution.touched = true;
                            }
                        }
                    }
                    ConflictPanelFocus::TheirsPanel => {
                        if let Some(resolution) = &mut conflict.line_level_resolution {
                            if let Some(included) = resolution.theirs_lines_included.get_mut(line_idx) {
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
            conflicts.line_selected += 1;
        }
    }
    Ok(())
}

fn handle_line_up(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.line_selected > 0 {
            conflicts.line_selected -= 1;
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
    if let Some(ref mut conflicts) = state.conflicts_state {
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

fn handle_edit_insert_char(_state: &mut AppState, _c: char) -> Result<()> {
    // Logique à implémenter
    Ok(())
}

fn handle_edit_backspace(_state: &mut AppState) -> Result<()> {
    // Logique à implémenter
    Ok(())
}

fn handle_edit_delete(_state: &mut AppState) -> Result<()> {
    Ok(())
}

fn handle_edit_cursor_up(_state: &mut AppState) -> Result<()> {
    Ok(())
}

fn handle_edit_cursor_down(_state: &mut AppState) -> Result<()> {
    Ok(())
}

fn handle_edit_cursor_left(_state: &mut AppState) -> Result<()> {
    Ok(())
}

fn handle_edit_cursor_right(_state: &mut AppState) -> Result<()> {
    Ok(())
}

fn handle_edit_newline(_state: &mut AppState) -> Result<()> {
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
            ConflictResolutionMode::File => {
                match conflicts.panel_focus {
                    ConflictPanelFocus::OursPanel => handle_accept_ours_file(state)?,
                    ConflictPanelFocus::TheirsPanel => handle_accept_theirs_file(state)?,
                    _ => {}
                }
            }
            ConflictResolutionMode::Block => {
                match conflicts.panel_focus {
                    ConflictPanelFocus::OursPanel => handle_accept_ours_block(state)?,
                    ConflictPanelFocus::TheirsPanel => handle_accept_theirs_block(state)?,
                    _ => {}
                }
            }
            ConflictResolutionMode::Line => {
                handle_toggle_line(state)?;
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
