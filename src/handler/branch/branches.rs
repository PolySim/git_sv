use crate::error::Result;
use crate::state::{AppState, ViewMode};
use crate::utils::{flash_error, flash_error_message, flash_success};

pub(super) fn handle_checkout(state: &mut AppState) -> Result<()> {
    let branch_info = if state.view_mode == ViewMode::Branches {
        state.branches_view_state.selected_branch_info()
    } else {
        None
    };

    if let Some((branch, selected)) = branch_info {
        if selected.is_remote() {
            state.set_flash_message(
                "Checkout impossible sur une branche distante. Créez d'abord une branche locale."
                    .to_string(),
            );
            return Ok(());
        }

        let branch_name = branch.name.clone();
        match crate::git::branch::checkout_branch(&state.repo.repo, &branch_name) {
            Ok(_) => {
                state.mark_dirty();
                state.set_flash_message(flash_success(format!(
                    "Branche '{}' check-out",
                    branch_name
                )));
            }
            Err(e) => {
                state.set_flash_message(flash_error("checkout", e));
            }
        }
    }
    Ok(())
}

pub(super) fn handle_delete(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    let selected_info = if state.view_mode == ViewMode::Branches {
        state.branches_view_state.selected_branch_info()
    } else {
        None
    };

    if let Some((branch, selected)) = selected_info {
        if selected.is_remote() {
            state.set_flash_message(crate::utils::flash_error_message(
                "suppression impossible sur une branche distante",
            ));
            return Ok(());
        }

        if branch.is_head {
            state.set_flash_message(crate::utils::flash_error_message(
                "impossible de supprimer la branche courante",
            ));
            return Ok(());
        }
        let branch_name = branch.name.clone();
        state.open_confirmation(ConfirmAction::BranchDelete(branch_name));
    }
    Ok(())
}

pub(super) fn handle_rename(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        if let Some((branch, selected)) = state.branches_view_state.selected_branch_info() {
            if selected.is_remote() {
                state.set_flash_message(crate::utils::flash_error_message(
                    "renommage impossible sur une branche distante",
                ));
                return Ok(());
            }

            let current_name = branch.name.clone();
            state.branches_view_state.focus = crate::state::BranchesFocus::Input;
            state.branches_view_state.input_action = Some(crate::state::InputAction::RenameBranch);
            state.branches_view_state.input_text = current_name;
            state.branches_view_state.input_cursor = state.branches_view_state.input_text.len();
        }
    }
    Ok(())
}

pub(super) fn handle_toggle_remote(state: &mut AppState) -> Result<()> {
    state.branches_view_state.toggle_remote();
    Ok(())
}

pub(super) fn handle_merge(state: &mut AppState) -> Result<()> {
    match crate::git::branch::list_all_branches(&state.repo.repo) {
        Ok((local, remote)) => {
            let current = state.current_branch.clone().unwrap_or_default();

            let mut branch_names: Vec<String> = local
                .iter()
                .filter(|b| b.name != current)
                .map(|b| b.name.clone())
                .collect();

            for b in &remote {
                branch_names.push(b.name.clone());
            }

            if branch_names.is_empty() {
                state.set_flash_message(crate::utils::flash_error_message(
                    "aucune autre branche disponible pour merge",
                ));
                return Ok(());
            }

            state.merge_picker = Some(crate::state::MergePickerState::new(branch_names));
        }
        Err(e) => {
            state.set_flash_message(flash_error_message(e));
        }
    }
    Ok(())
}
