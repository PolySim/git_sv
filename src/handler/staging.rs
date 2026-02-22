//! Handler pour les actions de staging.

use crate::error::Result;
use crate::state::{AppState, ViewMode, StagingFocus};
use crate::state::action::StagingAction;
use super::traits::{ActionHandler, HandlerContext};

/// Handler pour les opérations de staging.
pub struct StagingHandler;

impl ActionHandler for StagingHandler {
    type Action = StagingAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: StagingAction) -> Result<()> {
        match action {
            StagingAction::StageFile => handle_stage_file(ctx.state),
            StagingAction::UnstageFile => handle_unstage_file(ctx.state),
            StagingAction::StageAll => handle_stage_all(ctx.state),
            StagingAction::UnstageAll => handle_unstage_all(ctx.state),
            StagingAction::DiscardFile => handle_discard_file(ctx.state),
            StagingAction::DiscardAll => handle_discard_all(ctx.state),
            StagingAction::StartCommitMessage => handle_start_commit(ctx.state),
            StagingAction::ConfirmCommit => handle_confirm_commit(ctx.state),
            StagingAction::CancelCommit => handle_cancel_commit(ctx.state),
            StagingAction::SwitchFocus => handle_switch_focus(ctx.state),
            StagingAction::StashSelectedFile => handle_stash_selected_file(ctx.state),
            StagingAction::StashUnstagedFiles => handle_stash_unstaged_files(ctx.state),
        }
    }
}

fn handle_stage_file(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        if let Some(file) = state
            .staging_state
            .unstaged_files()
            .get(state.staging_state.unstaged_selected())
        {
            crate::git::commit::stage_file(&state.repo.repo, &file.path)?;
            state.mark_dirty();
            refresh_staging(state)?;
        }
    }
    Ok(())
}

fn handle_unstage_file(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        if let Some(file) = state
            .staging_state
            .staged_files()
            .get(state.staging_state.staged_selected())
        {
            crate::git::commit::unstage_file(&state.repo.repo, &file.path)?;
            state.mark_dirty();
            refresh_staging(state)?;
        }
    }
    Ok(())
}

fn handle_stage_all(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        crate::git::commit::stage_all(&state.repo.repo)?;
        state.mark_dirty();
        refresh_staging(state)?;
    }
    Ok(())
}

fn handle_unstage_all(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        crate::git::commit::unstage_all(&state.repo.repo)?;
        state.mark_dirty();
        refresh_staging(state)?;
    }
    Ok(())
}

fn handle_discard_file(state: &mut AppState) -> Result<()> {
    // Cette action nécessite une confirmation
    // Elle sera traitée par le confirm handler
    Ok(())
}

fn handle_discard_all(state: &mut AppState) -> Result<()> {
    // Cette action nécessite une confirmation
    // Elle sera traitée par le confirm handler
    Ok(())
}

fn handle_start_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.is_committing = true;
        state.staging_state.focus = StagingFocus::CommitMessage;
    }
    Ok(())
}

fn handle_confirm_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging
        && !state.staging_state.commit_message.is_empty()
    {
        let message = state.staging_state.commit_message.clone();
        crate::git::commit::create_commit(&state.repo.repo, &message)?;

        // Réinitialiser l'état du commit
        state.staging_state.is_committing = false;
        state.staging_state.commit_message.clear();
        state.staging_state.focus = StagingFocus::Unstaged;

        state.mark_dirty();
        refresh_staging(state)?;
    }
    Ok(())
}

fn handle_cancel_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.is_committing = false;
        state.staging_state.commit_message.clear();
        state.staging_state.focus = StagingFocus::Unstaged;
    }
    Ok(())
}

fn handle_switch_focus(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.focus = match state.staging_state.focus {
            StagingFocus::Unstaged => StagingFocus::Staged,
            StagingFocus::Staged => StagingFocus::Diff,
            StagingFocus::Diff => StagingFocus::Unstaged,
            StagingFocus::CommitMessage => StagingFocus::Unstaged,
        };
        load_staging_diff(state);
    }
    Ok(())
}

fn handle_stash_selected_file(state: &mut AppState) -> Result<()> {
    // Action placeholder - sera implémentée
    Ok(())
}

fn handle_stash_unstaged_files(state: &mut AppState) -> Result<()> {
    // Action placeholder - sera implémentée
    Ok(())
}

/// Rafraîchit l'état du staging depuis le repository.
pub fn refresh_staging(state: &mut AppState) -> Result<()> {
    let all_entries = state.repo.status()?;
    refresh_staging_with_entries(state, &all_entries)
}

/// Rafraîchit l'état du staging avec des entrées pré-filtrées.
pub fn refresh_staging_with_entries(
    state: &mut AppState,
    all_entries: &[crate::git::repo::StatusEntry],
) -> Result<()> {
    state.staging_state.set_staged_files(
        all_entries
            .iter()
            .filter(|e| e.is_staged())
            .cloned()
            .collect()
    );

    state.staging_state.set_unstaged_files(
        all_entries
            .iter()
            .filter(|e| e.is_unstaged())
            .cloned()
            .collect()
    );

    // Réajuster les sélections
    if state.staging_state.unstaged_selected()
        >= state.staging_state.unstaged_files().len()
    {
        let new_idx = state.staging_state.unstaged_files().len().saturating_sub(1);
        state.staging_state.set_unstaged_selected(new_idx);
    }
    if state.staging_state.staged_selected() >= state.staging_state.staged_files().len() {
        let new_idx = state.staging_state.staged_files().len().saturating_sub(1);
        state.staging_state.set_staged_selected(new_idx);
    }

    load_staging_diff(state);
    Ok(())
}

/// Charge le diff pour le fichier sélectionné dans le staging.
pub fn load_staging_diff(state: &mut AppState) {
    let selected_file = match state.staging_state.focus {
        StagingFocus::Unstaged => state
            .staging_state
            .unstaged_files()
            .get(state.staging_state.unstaged_selected()),
        StagingFocus::Staged => state
            .staging_state
            .staged_files()
            .get(state.staging_state.staged_selected()),
        _ => None,
    };

    if let Some(file) = selected_file {
        // Pour le working directory, on utilise Oid::zero() comme clé spéciale
        let cache_key = (git2::Oid::zero(), file.path.clone());

        // Essayer de récupérer du cache
        if let Some(cached_diff) = state.diff_cache.get(&cache_key) {
            state.staging_state.current_diff = Some(cached_diff.clone());
        } else {
            // Calculer et mettre en cache
            match crate::git::diff::working_dir_file_diff(&state.repo.repo, &file.path) {
                Ok(diff) => {
                    state.diff_cache.insert(cache_key, diff.clone());
                    state.staging_state.current_diff = Some(diff);
                }
                Err(_) => {
                    state.staging_state.current_diff = None;
                }
            }
        }
    } else {
        state.staging_state.current_diff = None;
    }
}
