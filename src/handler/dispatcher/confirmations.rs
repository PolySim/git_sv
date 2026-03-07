use crate::error::Result;
use crate::state::ViewMode;

use super::super::traits::HandlerContext;

/// Gère la confirmation d'une action destructive.
pub(super) fn handle_confirm_action(ctx: &mut HandlerContext) -> Result<()> {
    use crate::git::conflict::MergeResult;
    use crate::ui::confirm_dialog::ConfirmAction;

    if let Some(confirm_action) = ctx.state.pending_confirmation.clone() {
        match confirm_action {
            ConfirmAction::DiscardAll => {
                ctx.state.pending_confirmation = None;
                if let Err(e) = crate::git::discard::discard_all(&ctx.state.repo.repo) {
                    ctx.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    ctx.state
                        .set_flash_message("Modifications ignorées ✓".to_string());
                }
                ctx.state.mark_dirty();
            }
            ConfirmAction::DiscardFile(path) => {
                ctx.state.pending_confirmation = None;
                if let Err(e) = crate::git::discard::discard_file(&ctx.state.repo.repo, &path) {
                    ctx.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    ctx.state.set_flash_message(format!("{} ignoré ✓", path));
                }
                ctx.state.mark_dirty();
            }
            ConfirmAction::BranchDelete(name) => {
                ctx.state.pending_confirmation = None;
                if let Err(e) = crate::git::branch::delete_branch(&ctx.state.repo.repo, &name) {
                    ctx.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    ctx.state
                        .set_flash_message(format!("Branche {} supprimée ✓", name));
                }
                ctx.state.mark_dirty();
            }
            ConfirmAction::AbortMerge => {
                ctx.state.pending_confirmation = None;
                if let Err(e) = crate::git::conflict::abort_merge(&ctx.state.repo.repo) {
                    ctx.state.set_flash_message(format!("Erreur: {}", e));
                } else {
                    ctx.state.set_flash_message("Merge annulé ✓".to_string());
                    ctx.state.conflicts_state = None;
                    ctx.state.is_merging = false;
                }
                ctx.state.mark_dirty();
            }
            ConfirmAction::ResetSoft(oid) => {
                ctx.state.pending_confirmation = None;
                if let Err(e) = crate::git::commit::reset_to_commit(
                    &ctx.state.repo.repo,
                    oid,
                    git2::ResetType::Soft,
                ) {
                    ctx.state
                        .set_flash_message(format!("Erreur reset soft: {}", e));
                } else {
                    ctx.state
                        .set_flash_message(format!("Reset soft vers {oid:.7} effectué ✓"));
                    ctx.state.mark_dirty();
                }
            }
            ConfirmAction::ResetHard(oid) => {
                ctx.state.pending_confirmation = None;
                if let Err(e) = crate::git::commit::reset_to_commit(
                    &ctx.state.repo.repo,
                    oid,
                    git2::ResetType::Hard,
                ) {
                    ctx.state
                        .set_flash_message(format!("Erreur reset hard: {}", e));
                } else {
                    ctx.state
                        .set_flash_message(format!("Reset hard vers {oid:.7} effectué ✓"));
                    ctx.state.mark_dirty();
                }
            }
            ConfirmAction::StashDrop(index) => {
                ctx.state.pending_confirmation = None;
                if let Err(e) = crate::git::stash::drop_stash(&mut ctx.state.repo.repo, index) {
                    ctx.state
                        .set_flash_message(format!("Erreur suppression stash: {}", e));
                } else {
                    ctx.state
                        .set_flash_message(format!("Stash @{{{}}} supprimé ✓", index));
                    ctx.state.mark_dirty();
                }
            }
            ConfirmAction::WorktreeRemove(name) => {
                ctx.state.pending_confirmation = None;
                if let Err(e) = crate::git::worktree::remove_worktree(&ctx.state.repo.repo, &name) {
                    ctx.state
                        .set_flash_message(format!("Erreur suppression worktree: {}", e));
                } else {
                    ctx.state
                        .set_flash_message(format!("Worktree '{}' supprimé ✓", name));
                    ctx.state.mark_dirty();
                }
            }
            ConfirmAction::CherryPick(oid) => {
                ctx.state.pending_confirmation = None;
                match crate::git::commit::cherry_pick_with_result(&ctx.state.repo.repo, oid) {
                    Ok(MergeResult::Success) => {
                        ctx.state
                            .set_flash_message(format!("Cherry-pick {oid:.7} effectué ✓"));
                        ctx.state.mark_dirty();
                    }
                    Ok(MergeResult::Conflicts(conflicts)) => {
                        ctx.state.set_flash_message(format!(
                            "Conflits lors du cherry-pick ({} fichiers)",
                            conflicts.len()
                        ));
                        // Activer la vue conflits
                        let current = ctx
                            .state
                            .current_branch
                            .clone()
                            .unwrap_or_else(|| "HEAD".to_string());
                        ctx.state.conflicts_state = Some(crate::state::ConflictsState::new(
                            conflicts,
                            format!("cherry-pick {oid:.7}"),
                            current,
                            format!("{:.7}", oid),
                        ));
                        ctx.state.view_mode = ViewMode::Conflicts;
                    }
                    Ok(_) => {
                        // UpToDate ou FastForward - ne devrait pas arriver en cherry-pick
                        ctx.state
                            .set_flash_message("Cherry-pick effectué ✓".to_string());
                        ctx.state.mark_dirty();
                    }
                    Err(e) => {
                        ctx.state
                            .set_flash_message(format!("Erreur cherry-pick: {}", e));
                    }
                }
            }
            ConfirmAction::MergeBranch(source, target) => {
                ctx.state.pending_confirmation = None;
                // Note: le merge devrait être fait sur la branche cible,
                // mais comme on est déjà dessus (par définition), on merge juste la source
                match crate::git::merge::merge_branch_with_result(&ctx.state.repo.repo, &source) {
                    Ok(MergeResult::UpToDate) => {
                        ctx.state
                            .set_flash_message(format!("Branche '{}' est déjà à jour", source));
                    }
                    Ok(MergeResult::FastForward) => {
                        ctx.state
                            .set_flash_message(format!("Fast-forward vers '{}'", source));
                        ctx.state.mark_dirty();
                    }
                    Ok(MergeResult::Success) => {
                        ctx.state.set_flash_message(format!(
                            "Branche '{}' mergée dans '{}' avec succès",
                            source, target
                        ));
                        ctx.state.mark_dirty();
                    }
                    Ok(MergeResult::Conflicts(conflicts)) => {
                        ctx.state.set_flash_message(format!(
                            "Conflits lors du merge avec '{}' ({} fichiers)",
                            source,
                            conflicts.len()
                        ));
                        // Activer la vue conflits
                        let current = ctx
                            .state
                            .current_branch
                            .clone()
                            .unwrap_or_else(|| "HEAD".to_string());
                        ctx.state.conflicts_state = Some(crate::state::ConflictsState::new(
                            conflicts,
                            format!("merge {}", source),
                            current,
                            source.clone(),
                        ));
                        ctx.state.view_mode = ViewMode::Conflicts;
                    }
                    Err(e) => {
                        ctx.state.set_flash_message(format!("Erreur merge: {}", e));
                    }
                }
            }
        }
    }
    Ok(())
}
