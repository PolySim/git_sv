use super::super::traits::HandlerContext;
use crate::error::Result;
use crate::utils::{flash_error, flash_error_message, flash_success};

/// Gère la confirmation d'une action destructive.
pub(super) fn handle_confirm_action(ctx: &mut HandlerContext) -> Result<()> {
    use crate::git::conflict::MergeResult;
    use crate::ui::confirm_dialog::ConfirmAction;

    if let Some(confirm_action) = ctx.state.ui.pending_confirmation.take() {
        match confirm_action {
            ConfirmAction::DiscardAll => {
                if let Err(e) = crate::git::discard::discard_all(&ctx.state.repo.repo) {
                    ctx.state.set_flash_message(flash_error_message(e));
                } else {
                    ctx.state
                        .set_flash_message(flash_success("Modifications ignorées"));
                }
                ctx.state.mark_dirty();
            }
            ConfirmAction::DiscardFile(path) => {
                if let Err(e) = crate::git::discard::discard_file(&ctx.state.repo.repo, &path) {
                    ctx.state.set_flash_message(flash_error_message(e));
                } else {
                    ctx.state
                        .set_flash_message(flash_success(format!("{} ignoré", path)));
                }
                ctx.state.mark_dirty();
            }
            ConfirmAction::BranchDelete(name) => {
                if let Err(e) = crate::git::branch::delete_branch(&ctx.state.repo.repo, &name) {
                    ctx.state.set_flash_message(flash_error_message(e));
                } else {
                    ctx.state
                        .set_flash_message(flash_success(format!("Branche {} supprimée", name)));
                }
                ctx.state.mark_dirty();
            }
            ConfirmAction::AbortMerge => {
                if let Err(e) = crate::git::conflict::abort_merge(&ctx.state.repo.repo) {
                    ctx.state.set_flash_message(flash_error_message(e));
                } else {
                    ctx.state.set_flash_message(flash_success("Merge annulé"));
                    ctx.state.conflicts_state = None;
                    ctx.state.ui.is_merging = false;
                }
                ctx.state.mark_dirty();
            }
            ConfirmAction::ResetSoft(oid) => {
                if let Err(e) = crate::git::commit::reset_to_commit(
                    &ctx.state.repo.repo,
                    oid,
                    git2::ResetType::Soft,
                ) {
                    ctx.state.set_flash_message(flash_error("reset soft", e));
                } else {
                    ctx.state.set_flash_message(flash_success(format!(
                        "Reset soft vers {oid:.7} effectué"
                    )));
                    ctx.state.mark_dirty();
                }
            }
            ConfirmAction::ResetHard(oid) => {
                if let Err(e) = crate::git::commit::reset_to_commit(
                    &ctx.state.repo.repo,
                    oid,
                    git2::ResetType::Hard,
                ) {
                    ctx.state.set_flash_message(flash_error("reset hard", e));
                } else {
                    ctx.state.set_flash_message(flash_success(format!(
                        "Reset hard vers {oid:.7} effectué"
                    )));
                    ctx.state.mark_dirty();
                }
            }
            ConfirmAction::StashDrop(index) => {
                if let Err(e) = crate::git::stash::drop_stash(&mut ctx.state.repo.repo, index) {
                    ctx.state
                        .set_flash_message(flash_error("suppression stash", e));
                } else {
                    ctx.state
                        .set_flash_message(flash_success(format!("Stash @{{{}}} supprimé", index)));
                    ctx.state.mark_dirty();
                }
            }
            ConfirmAction::WorktreeRemove(name) => {
                if let Err(e) = crate::git::worktree::remove_worktree(&ctx.state.repo.repo, &name) {
                    ctx.state
                        .set_flash_message(flash_error("suppression worktree", e));
                } else {
                    ctx.state
                        .set_flash_message(flash_success(format!("Worktree '{}' supprimé", name)));
                    ctx.state.mark_dirty();
                }
            }
            ConfirmAction::CherryPick(oid) => {
                match crate::git::commit::cherry_pick_with_result(&ctx.state.repo.repo, oid) {
                    Ok(MergeResult::Success) => {
                        ctx.state.set_flash_message(flash_success(format!(
                            "Cherry-pick {oid:.7} effectué"
                        )));
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
                        ctx.state.open_conflicts(crate::state::ConflictsState::new(
                            conflicts,
                            format!("cherry-pick {oid:.7}"),
                            current,
                            format!("{:.7}", oid),
                        ));
                    }
                    Ok(_) => {
                        // UpToDate ou FastForward - ne devrait pas arriver en cherry-pick
                        ctx.state
                            .set_flash_message(flash_success("Cherry-pick effectué"));
                        ctx.state.mark_dirty();
                    }
                    Err(e) => {
                        ctx.state.set_flash_message(flash_error("cherry-pick", e));
                    }
                }
            }
        }
    }
    Ok(())
}
