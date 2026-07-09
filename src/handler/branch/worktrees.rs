use crate::error::Result;
use crate::state::{AppState, BranchesSection, ViewMode};
use crate::utils::{flash_error, flash_error_message, flash_success};

pub(super) fn handle_open_worktrees(state: &mut AppState) -> Result<()> {
    state.branches_view_state.section = BranchesSection::Worktrees;
    state.enter_view(ViewMode::Branches);
    Ok(())
}

pub(super) fn handle_worktree_switch(state: &mut AppState) -> Result<()> {
    let selected = state.branches_view_state.worktree_selected();
    let Some(worktree) = state.branches_view_state.worktrees.get(selected) else {
        return Ok(());
    };
    if worktree.is_current {
        state.set_flash_message(flash_success("Worktree deja actif"));
        return Ok(());
    }

    let name = worktree.name.clone();
    let path = worktree.path.clone();
    if let Err(error) = state.switch_repository(&path) {
        state.set_flash_message(flash_error("ouverture worktree", error));
        return Ok(());
    }

    state.set_flash_message(flash_success(format!("Worktree '{}' ouvert", name)));
    Ok(())
}

pub(super) fn handle_worktree_remove(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    let selected = state.branches_view_state.worktree_selected();
    if let Some(worktree) = state.branches_view_state.worktrees.get(selected) {
        if worktree.is_main || worktree.is_current {
            state.set_flash_message(flash_error_message(
                "Suppression impossible: le worktree principal ou actif ne peut pas etre supprime",
            ));
            return Ok(());
        }
        let name = worktree.name.clone();
        state.open_confirmation(ConfirmAction::WorktreeRemove(name));
    }
    Ok(())
}
