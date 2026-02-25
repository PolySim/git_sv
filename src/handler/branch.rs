//! Handler pour les actions sur les branches.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::BranchAction;
use crate::state::{AppState, BranchesSection, ViewMode};

/// Handler pour les opérations sur les branches.
pub struct BranchHandler;

impl ActionHandler for BranchHandler {
    type Action = BranchAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: BranchAction) -> Result<()> {
        match action {
            BranchAction::List => handle_list(ctx.state),
            BranchAction::Checkout => handle_checkout(ctx.state),
            BranchAction::Create => handle_create(ctx.state),
            BranchAction::Delete => handle_delete(ctx.state),
            BranchAction::Rename => handle_rename(ctx.state),
            BranchAction::ToggleRemote => handle_toggle_remote(ctx.state),
            BranchAction::Merge => handle_merge(ctx.state),
            BranchAction::StashSave => handle_stash_save(ctx.state),
            BranchAction::StashApply => handle_stash_apply(ctx.state),
            BranchAction::StashPop => handle_stash_pop(ctx.state),
            BranchAction::StashDrop => handle_stash_drop(ctx.state),
            BranchAction::WorktreeCreate => handle_worktree_create(ctx.state),
            BranchAction::WorktreeRemove => handle_worktree_remove(ctx.state),
            BranchAction::NextSection => handle_next_section(ctx.state),
            BranchAction::PrevSection => handle_prev_section(ctx.state),
            BranchAction::ConfirmInput => Ok(()), // Géré par le handler d'édition
            BranchAction::CancelInput => Ok(()),  // Géré par le handler d'édition
        }
    }
}

fn handle_list(state: &mut AppState) -> Result<()> {
    if matches!(state.view_mode, ViewMode::Graph | ViewMode::Branches) {
        state.show_branch_panel = !state.show_branch_panel;
        if state.show_branch_panel {
            match crate::git::branch::list_all_branches(&state.repo.repo) {
                Ok((local, remote)) => {
                    // Legacy (pour le panneau overlay en Graph view)
                    state.branches = local.clone();
                    state.branch_selected = 0;
                    // Nouveau système (pour la vue Branches)
                    state.branches_view_state.local_branches.set_items(local);
                    state.branches_view_state.remote_branches.set_items(remote);
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur: {}", e));
                }
            }
        }
    }
    Ok(())
}

fn handle_checkout(state: &mut AppState) -> Result<()> {
    let branch_name = if state.view_mode == ViewMode::Branches {
        // Lire depuis le nouvel état BranchesViewState
        state
            .branches_view_state
            .selected_branch()
            .map(|b| b.name.clone())
    } else if state.view_mode == ViewMode::Graph && state.show_branch_panel {
        // Legacy: panel overlay dans la vue Graph
        state
            .branches
            .get(state.branch_selected)
            .map(|b| b.name.clone())
    } else {
        None
    };

    if let Some(branch_name) = branch_name {
        match crate::git::branch::checkout_branch(&state.repo.repo, &branch_name) {
            Ok(_) => {
                // Fermer le panel si applicable
                if state.show_branch_panel {
                    state.show_branch_panel = false;
                }
                state.mark_dirty();
                state.set_flash_message(format!("Branche '{}' check-out ✓", branch_name));
            }
            Err(e) => {
                state.set_flash_message(format!("Erreur checkout: {}", e));
            }
        }
    }
    Ok(())
}

fn handle_create(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = crate::state::BranchesFocus::Input;
        state.branches_view_state.input_action = Some(crate::state::InputAction::CreateBranch);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}

fn handle_delete(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if let Some(branch_info) = state.branches.get(state.branch_selected) {
        let branch_name = branch_info.name.clone();
        state.pending_confirmation = Some(ConfirmAction::BranchDelete(branch_name));
    }
    Ok(())
}

fn handle_rename(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        if let Some(branch) = state.branches_view_state.selected_branch() {
            let current_name = branch.name.clone();
            state.branches_view_state.focus = crate::state::BranchesFocus::Input;
            state.branches_view_state.input_action = Some(crate::state::InputAction::RenameBranch);
            state.branches_view_state.input_text = current_name;
            state.branches_view_state.input_cursor = state.branches_view_state.input_text.len();
        }
    }
    Ok(())
}

fn handle_toggle_remote(state: &mut AppState) -> Result<()> {
    state.branches_view_state.show_remote = !state.branches_view_state.show_remote;
    Ok(())
}

fn handle_merge(state: &mut AppState) -> Result<()> {
    // Charger la liste des branches pour le merge picker
    match crate::git::branch::list_all_branches(&state.repo.repo) {
        Ok((local, remote)) => {
            let current = state.current_branch.clone().unwrap_or_default();

            // Construire la liste des branches (exclure la branche courante)
            let mut branch_names: Vec<String> = local
                .iter()
                .filter(|b| b.name != current)
                .map(|b| b.name.clone())
                .collect();

            // Ajouter les branches remote
            for b in &remote {
                branch_names.push(b.name.clone());
            }

            if branch_names.is_empty() {
                state.set_flash_message("Aucune autre branche disponible pour merge".to_string());
                return Ok(());
            }

            state.merge_picker = Some(crate::state::MergePickerState::new(branch_names));
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur: {}", e));
        }
    }
    Ok(())
}

fn handle_stash_save(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = crate::state::BranchesFocus::Input;
        state.branches_view_state.input_action = Some(crate::state::InputAction::SaveStash);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}

fn handle_stash_apply(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected).cloned() {
            let index = stash.index;
            match crate::git::stash::apply_stash(&mut state.repo.repo, index) {
                Ok(_) => {
                    state.mark_dirty();
                    state.set_flash_message("Stash appliqué ✓".to_string());
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur: {}", e));
                }
            }
        }
    }
    Ok(())
}

fn handle_stash_pop(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected).cloned() {
            let index = stash.index;
            match crate::git::stash::pop_stash(&mut state.repo.repo, index) {
                Ok(_) => {
                    state.mark_dirty();
                    state.set_flash_message("Stash pop ✓".to_string());
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur: {}", e));
                }
            }
        }
    }
    Ok(())
}

fn handle_stash_drop(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Branches {
        let selected = state.branches_view_state.stash_selected();
        if let Some(stash) = state.branches_view_state.stashes.get(selected) {
            let index = stash.index;
            state.pending_confirmation = Some(ConfirmAction::StashDrop(index));
        }
    }
    Ok(())
}

fn handle_worktree_create(_state: &mut AppState) -> Result<()> {
    // Ouvre un input pour créer un worktree (géré par le handler d'édition)
    Ok(())
}

fn handle_worktree_remove(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    let selected = state.branches_view_state.worktree_selected();
    if let Some(worktree) = state.branches_view_state.worktrees.get(selected) {
        let path = worktree.path.clone();
        state.pending_confirmation = Some(ConfirmAction::WorktreeRemove(path));
    }
    Ok(())
}

fn handle_next_section(state: &mut AppState) -> Result<()> {
    state.branches_view_state.section = match state.branches_view_state.section {
        BranchesSection::Branches => BranchesSection::Worktrees,
        BranchesSection::Worktrees => BranchesSection::Stashes,
        BranchesSection::Stashes => BranchesSection::Branches,
    };
    Ok(())
}

fn handle_prev_section(state: &mut AppState) -> Result<()> {
    state.branches_view_state.section = match state.branches_view_state.section {
        BranchesSection::Branches => BranchesSection::Stashes,
        BranchesSection::Worktrees => BranchesSection::Branches,
        BranchesSection::Stashes => BranchesSection::Worktrees,
    };
    Ok(())
}

fn handle_confirm_input(state: &mut AppState) -> Result<()> {
    let input = state.branches_view_state.input_text.trim().to_string();
    if input.is_empty() {
        state.branches_view_state.focus = crate::state::BranchesFocus::List;
        state.branches_view_state.input_action = None;
        return Ok(());
    }

    match state.branches_view_state.input_action {
        Some(crate::state::InputAction::CreateBranch) => {
            match crate::git::branch::create_branch(&state.repo.repo, &input) {
                Ok(_) => {
                    state.set_flash_message(format!("Branche '{}' créée ✓", input));
                    state.mark_dirty();
                }
                Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
            }
        }
        Some(crate::state::InputAction::RenameBranch) => {
            if let Some(branch) = state.branches_view_state.selected_branch() {
                let old_name = branch.name.clone();
                match crate::git::branch::rename_branch(&state.repo.repo, &old_name, &input) {
                    Ok(_) => {
                        state.set_flash_message(format!("Branche renommée → '{}' ✓", input));
                        state.mark_dirty();
                    }
                    Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
                }
            }
        }
        Some(crate::state::InputAction::SaveStash) => {
            match crate::git::stash::save_stash(&mut state.repo.repo, Some(&input)) {
                Ok(_) => {
                    state.set_flash_message(format!("Stash créé: {} ✓", input));
                    state.mark_dirty();
                }
                Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
            }
        }
        Some(crate::state::InputAction::CreateWorktree) => {
            // Le format attendu est "nom chemin [branche]"
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let path = parts[1];
                let branch = parts.get(2).copied();
                match crate::git::worktree::create_worktree(&state.repo.repo, name, path, branch) {
                    Ok(_) => {
                        state.set_flash_message(format!("Worktree '{}' créé ✓", name));
                        state.mark_dirty();
                    }
                    Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
                }
            } else {
                state.set_flash_message("Format: nom chemin [branche]".to_string());
            }
        }
        None => {}
    }

    state.branches_view_state.focus = crate::state::BranchesFocus::List;
    state.branches_view_state.input_action = None;
    state.branches_view_state.input_text.clear();
    state.branches_view_state.input_cursor = 0;
    Ok(())
}

fn handle_cancel_input(state: &mut AppState) -> Result<()> {
    state.branches_view_state.focus = crate::state::BranchesFocus::List;
    state.branches_view_state.input_action = None;
    state.branches_view_state.input_text.clear();
    state.branches_view_state.input_cursor = 0;
    Ok(())
}
