//! Handler pour les actions sur les branches.

use crate::error::Result;
use crate::state::{AppState, ViewMode, BranchesSection};
use crate::state::action::BranchAction;
use super::traits::{ActionHandler, HandlerContext};

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
            BranchAction::CancelInput => Ok(()), // Géré par le handler d'édition
        }
    }
}

fn handle_list(state: &mut AppState) -> Result<()> {
    if matches!(state.view_mode, ViewMode::Graph | ViewMode::Branches) {
        state.show_branch_panel = !state.show_branch_panel;
        if state.show_branch_panel {
            // Recharger les branches
            match crate::git::branch::list_all_branches(&state.repo.repo) {
                Ok((local, _remote)) => {
                    // Convertir BranchInfo en String (noms uniquement)
                    state.branches = local.into_iter().collect();
                    state.branch_selected = 0;
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
    if state.view_mode == ViewMode::Graph && state.show_branch_panel {
        let branch_name = state.branches.get(state.branch_selected).map(|b| b.name.clone());
        if let Some(branch_name) = branch_name {
            match crate::git::branch::checkout_branch(&state.repo.repo, &branch_name) {
                Ok(_) => {
                    state.show_branch_panel = false;
                    state.mark_dirty();
                    state.set_flash_message(format!("Branche '{}' check-out ✓", branch_name));
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur checkout: {}", e));
                }
            }
        }
    }
    Ok(())
}

fn handle_create(_state: &mut AppState) -> Result<()> {
    // Ouvre un input pour créer une branche (géré par le handler d'édition)
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

fn handle_rename(_state: &mut AppState) -> Result<()> {
    // Ouvre un input pour renommer (géré par le handler d'édition)
    Ok(())
}

fn handle_toggle_remote(state: &mut AppState) -> Result<()> {
    state.branches_view_state.show_remote = !state.branches_view_state.show_remote;
    Ok(())
}

fn handle_merge(_state: &mut AppState) -> Result<()> {
    // Ouvre le sélecteur de merge
    Ok(())
}

fn handle_stash_save(_state: &mut AppState) -> Result<()> {
    // Ouvre un input pour le message du stash (géré par le handler d'édition)
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
