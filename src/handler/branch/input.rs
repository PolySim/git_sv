use crate::error::Result;
use crate::state::{AppState, BranchesFocus, BranchesSection, InputAction, ViewMode};
use crate::utils::{flash_error, flash_error_message, flash_success};

pub(super) fn handle_create(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateBranch);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}

pub(super) fn handle_stash_save(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::SaveStash);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}

pub(super) fn handle_worktree_create(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateWorktree);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}

pub(super) fn handle_next_section(state: &mut AppState) -> Result<()> {
    state.branches_view_state.section = match state.branches_view_state.section {
        BranchesSection::Branches => BranchesSection::Worktrees,
        BranchesSection::Worktrees => BranchesSection::Stashes,
        BranchesSection::Stashes => BranchesSection::Branches,
    };
    Ok(())
}

pub(super) fn handle_prev_section(state: &mut AppState) -> Result<()> {
    state.branches_view_state.section = match state.branches_view_state.section {
        BranchesSection::Branches => BranchesSection::Stashes,
        BranchesSection::Worktrees => BranchesSection::Branches,
        BranchesSection::Stashes => BranchesSection::Worktrees,
    };
    Ok(())
}

pub(super) fn handle_confirm_input(state: &mut AppState) -> Result<()> {
    let input = state.branches_view_state.input_text.trim().to_string();
    if input.is_empty() {
        return handle_cancel_input(state);
    }

    match state.branches_view_state.input_action {
        Some(InputAction::CreateBranch) => {
            match crate::git::branch::create_branch(&state.repo.repo, &input) {
                Ok(_) => {
                    state.set_flash_message(flash_success(format!("Branche '{}' créée", input)));
                    state.mark_dirty();
                }
                Err(e) => state.set_flash_message(flash_error_message(e)),
            }
        }
        Some(InputAction::RenameBranch) => {
            if let Some(branch) = state.branches_view_state.selected_branch() {
                let old_name = branch.name.clone();
                match crate::git::branch::rename_branch(&state.repo.repo, &old_name, &input) {
                    Ok(_) => {
                        state.set_flash_message(flash_success(format!(
                            "Branche renommée → '{}'",
                            input
                        )));
                        state.mark_dirty();
                    }
                    Err(e) => state.set_flash_message(flash_error_message(e)),
                }
            }
        }
        Some(InputAction::SaveStash) => {
            match crate::git::stash::save_stash(&mut state.repo.repo, Some(&input)) {
                Ok(_) => {
                    state.set_flash_message(flash_success(format!("Stash créé: {}", input)));
                    state.mark_dirty();
                }
                Err(e) => state.set_flash_message(flash_error_message(e)),
            }
        }
        Some(InputAction::CreateWorktree) => {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() < 2 {
                state.set_flash_message(crate::utils::flash_error_message(
                    "format attendu: nom chemin [branche]",
                ));
            } else {
                let name = parts[0];
                let path = parts[1];
                let branch = parts.get(2).copied();

                if name.is_empty() {
                    state.set_flash_message(crate::utils::flash_error_message(
                        "le nom du worktree ne peut pas être vide",
                    ));
                } else if path.is_empty() {
                    state.set_flash_message(crate::utils::flash_error_message(
                        "le chemin du worktree ne peut pas être vide",
                    ));
                } else {
                    let worktree_exists = state
                        .branches_view_state
                        .worktrees
                        .iter()
                        .any(|w| w.name == name);

                    if worktree_exists {
                        state.set_flash_message(crate::utils::flash_error_message(format!(
                            "un worktree '{}' existe déjà",
                            name
                        )));
                    } else {
                        match crate::git::worktree::create_worktree(
                            &state.repo.repo,
                            name,
                            path,
                            branch,
                        ) {
                            Ok(_) => {
                                state.set_flash_message(flash_success(format!(
                                    "Worktree '{}' créé",
                                    name
                                )));
                                state.mark_dirty();

                                if let Ok(worktrees) =
                                    crate::git::worktree::list_worktrees(&state.repo.repo)
                                {
                                    state.branches_view_state.worktrees.set_items(worktrees);

                                    if let Some(idx) = state
                                        .branches_view_state
                                        .worktrees
                                        .iter()
                                        .position(|w| w.name == name)
                                    {
                                        state.branches_view_state.worktrees.select(idx);
                                    }
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("{}", e);
                                if error_msg.contains("exists") || error_msg.contains("déjà") {
                                    state.set_flash_message(crate::utils::flash_error_message(
                                        format!("le chemin '{}' existe déjà", path),
                                    ));
                                } else if error_msg.contains("invalid")
                                    || error_msg.contains("invalide")
                                {
                                    state.set_flash_message(crate::utils::flash_error_message(
                                        format!("chemin invalide '{}'", path),
                                    ));
                                } else if error_msg.contains("branch")
                                    || error_msg.contains("branche")
                                {
                                    state.set_flash_message(crate::utils::flash_error_message(
                                        format!("branche '{}' inexistante", branch.unwrap_or("")),
                                    ));
                                } else {
                                    state.set_flash_message(flash_error("création worktree", e));
                                }
                            }
                        }
                    }
                }
            }
        }
        None => {}
    }

    handle_cancel_input(state)
}

pub(super) fn handle_cancel_input(state: &mut AppState) -> Result<()> {
    state.branches_view_state.focus = BranchesFocus::List;
    state.branches_view_state.input_action = None;
    state.branches_view_state.input_text.clear();
    state.branches_view_state.input_cursor = 0;
    Ok(())
}
