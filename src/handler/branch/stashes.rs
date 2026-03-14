use crate::error::Result;
use crate::state::{AppState, BranchesSection, ViewMode};
use crate::utils::{flash_error_message, flash_success};

pub(super) fn handle_stash_apply(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected).cloned() {
            let index = stash.index;
            match crate::git::stash::apply_stash(&mut state.repo.repo, index) {
                Ok(_) => {
                    state.mark_dirty();
                    state.set_flash_message(flash_success("Stash appliqué"));
                }
                Err(e) => {
                    state.set_flash_message(flash_error_message(e));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn handle_stash_pop(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected).cloned() {
            let index = stash.index;
            match crate::git::stash::pop_stash(&mut state.repo.repo, index) {
                Ok(_) => {
                    state.mark_dirty();
                    state.set_flash_message(flash_success("Stash pop"));
                }
                Err(e) => {
                    state.set_flash_message(flash_error_message(e));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn handle_stash_drop(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected) {
            state.open_confirmation(ConfirmAction::StashDrop(stash.index));
        }
    }
    Ok(())
}

pub(super) fn handle_stash_file_next(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.section == BranchesSection::Stashes
    {
        if let Some(stash) = state.branches_view_state.stashes.selected_item() {
            let file_count = stash.files.len();
            if file_count > 0 {
                let idx = &mut state.branches_view_state.stash_file_selected;
                *idx = (*idx + 1).min(file_count - 1);
                load_stash_file_diff(state)?;
            }
        }
    }
    Ok(())
}

pub(super) fn handle_stash_file_prev(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.section == BranchesSection::Stashes
    {
        let idx = &mut state.branches_view_state.stash_file_selected;
        *idx = idx.saturating_sub(1);
        load_stash_file_diff(state)?;
    }
    Ok(())
}

pub fn load_stash_file_diff(state: &mut AppState) -> Result<()> {
    if let Some(stash) = state.branches_view_state.stashes.selected_item() {
        let idx = state.branches_view_state.stash_file_selected;
        if let Some(file) = stash.files.get(idx) {
            match state.repo.stash_file_diff(stash.oid, &file.path) {
                Ok(diff) => {
                    state.branches_view_state.stash_file_diff = Some(diff);
                }
                Err(e) => {
                    state.set_flash_message(crate::utils::flash_error("chargement diff", e));
                    state.branches_view_state.stash_file_diff = None;
                }
            }
        }
    }
    Ok(())
}
