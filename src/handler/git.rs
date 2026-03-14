//! Handler pour les actions Git (remote, blame, cherry-pick, etc.).

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::GitAction;
use crate::state::{AppState, BlameState, FocusPanel, StagingFocus, ViewMode};
use crate::utils::{flash_error, flash_error_message, flash_success};

/// Handler pour les opérations Git.
pub struct GitHandler;

impl ActionHandler for GitHandler {
    type Action = GitAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: GitAction) -> Result<()> {
        match action {
            GitAction::Push => handle_push(ctx.state),
            GitAction::ForcePush => handle_force_push(ctx.state),
            GitAction::Pull => handle_pull(ctx.state),
            GitAction::Fetch => handle_fetch(ctx.state),
            GitAction::CherryPick => handle_cherry_pick(ctx.state),
            GitAction::AmendCommit => handle_amend_commit(ctx.state),
            GitAction::OpenBlame => handle_open_blame(ctx.state),
            GitAction::CloseBlame => handle_close_blame(ctx.state),
            GitAction::JumpToBlameCommit => handle_jump_to_blame_commit(ctx.state),
            GitAction::CommitPrompt => handle_commit_prompt(ctx.state),
            GitAction::StashPrompt => handle_stash_prompt(ctx.state),
            GitAction::MergePrompt => handle_merge_prompt(ctx.state),
            GitAction::ResetPrompt => handle_reset_prompt(ctx.state),
            GitAction::AbortMerge => handle_abort_merge(ctx.state),
        }
    }
}

fn handle_push(state: &mut AppState) -> Result<()> {
    match crate::git::remote::has_remote(&state.repo.repo) {
        Ok(true) => match crate::git::remote::push_current_branch(&state.repo.repo) {
            Ok(success) => {
                state.set_flash_message(success.flash_message());
            }
            Err(e) => {
                state.set_flash_message(flash_error("lors du push", e));
            }
        },
        Ok(false) => {
            state.set_flash_message(flash_error_message("aucun remote configuré"));
        }
        Err(e) => {
            state.set_flash_message(flash_error_message(e));
        }
    }
    Ok(())
}

fn handle_force_push(state: &mut AppState) -> Result<()> {
    match crate::git::remote::has_remote(&state.repo.repo) {
        Ok(true) => match crate::git::remote::force_push_current_branch(&state.repo.repo) {
            Ok(success) => {
                state.set_flash_message(success.flash_message());
            }
            Err(e) => {
                state.set_flash_message(flash_error("lors du force push", e));
            }
        },
        Ok(false) => {
            state.set_flash_message(flash_error_message("aucun remote configuré"));
        }
        Err(e) => {
            state.set_flash_message(flash_error_message(e));
        }
    }
    Ok(())
}

fn handle_pull(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::MergeResult;
    use crate::git::remote::flash_message_for_pull_result;
    use crate::state::ConflictsState;

    match crate::git::remote::has_remote(&state.repo.repo) {
        Ok(true) => match crate::git::remote::pull_current_branch_with_result(&state.repo.repo) {
            Ok(MergeResult::UpToDate) => {
                state.set_flash_message(
                    flash_message_for_pull_result(&MergeResult::UpToDate).unwrap(),
                );
            }
            Ok(MergeResult::FastForward) => {
                state.set_flash_message(
                    flash_message_for_pull_result(&MergeResult::FastForward).unwrap(),
                );
                state.mark_dirty();
            }
            Ok(MergeResult::Success) => {
                state.set_flash_message(
                    flash_message_for_pull_result(&MergeResult::Success).unwrap(),
                );
                state.mark_dirty();
            }
            Ok(MergeResult::Conflicts(files)) => {
                let ours_name = crate::git::conflict::get_current_branch_name(&state.repo.repo);
                let theirs_name = format!(
                    "origin/{}",
                    state
                        .current_branch
                        .clone()
                        .unwrap_or_else(|| "HEAD".to_string())
                );
                state.conflicts_state = Some(ConflictsState::new(
                    files,
                    "Pull depuis origin".to_string(),
                    ours_name,
                    theirs_name,
                ));
                state.view_mode = ViewMode::Conflicts;
                state.set_flash_message(flash_error_message(
                    "conflits lors du pull - résolution requise",
                ));
            }
            Err(e) => {
                state.set_flash_message(flash_error("lors du pull", e));
            }
        },
        Ok(false) => {
            state.set_flash_message(flash_error_message("aucun remote configuré"));
        }
        Err(e) => {
            state.set_flash_message(flash_error_message(e));
        }
    }
    Ok(())
}

fn handle_fetch(state: &mut AppState) -> Result<()> {
    match crate::git::remote::has_remote(&state.repo.repo) {
        Ok(true) => match crate::git::remote::fetch_all(&state.repo.repo) {
            Ok(_) => {
                let remote_name = crate::git::remote::get_default_remote(&state.repo.repo)
                    .unwrap_or_else(|_| "origin".to_string());
                state.set_flash_message(
                    crate::git::remote::FetchSuccess { remote_name }.flash_message(),
                );
                state.mark_dirty();
            }
            Err(e) => {
                state.set_flash_message(flash_error("lors du fetch", e));
            }
        },
        Ok(false) => {
            state.set_flash_message(flash_error_message("aucun remote configuré"));
        }
        Err(e) => {
            state.set_flash_message(flash_error_message(e));
        }
    }
    Ok(())
}

fn handle_cherry_pick(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if !matches!(state.view_mode, ViewMode::Graph) {
        return Ok(());
    }

    // Utiliser l'API unifiée pour obtenir le commit sélectionné
    let commit_oid = if let Some(commit) = state.selected_commit() {
        commit.oid
    } else {
        state.set_flash_message(flash_error_message("aucun commit sélectionné"));
        return Ok(());
    };

    state.open_confirmation(ConfirmAction::CherryPick(commit_oid));

    Ok(())
}

fn handle_amend_commit(state: &mut AppState) -> Result<()> {
    use crate::state::StagingFocus;

    if !matches!(state.view_mode, ViewMode::Staging) {
        return Ok(());
    }

    let commit_message = {
        let head_commit = state.repo.repo.head()?.peel_to_commit()?;
        head_commit.message().unwrap_or("").to_string()
    };

    state.staging_state.commit_message = commit_message;
    state.staging_state.cursor_position = state.staging_state.commit_message.len();
    state.staging_state.is_committing = true;
    state.staging_state.is_amending = true;
    state.staging_state.focus = StagingFocus::CommitMessage;

    state.set_flash_message("Mode amendement activé - éditez le message et validez".to_string());

    Ok(())
}

fn handle_open_blame(state: &mut AppState) -> Result<()> {
    if !matches!(state.view_mode, ViewMode::Graph) {
        return Ok(());
    }

    if state.focus != FocusPanel::BottomLeft {
        return Ok(());
    }

    // Utiliser l'API unifiée pour accéder aux fichiers
    if state.graph_view.commit_files.is_empty() {
        state.set_flash_message(flash_error_message("aucun fichier sélectionné"));
        return Ok(());
    }

    let selected_file = match state
        .graph_view
        .commit_files
        .get(state.graph_view.file_selected_index)
    {
        Some(f) => f,
        None => {
            state.set_flash_message(flash_error_message("index de fichier invalide"));
            return Ok(());
        }
    };
    let file_path = selected_file.path.clone();

    // Utiliser l'API unifiée pour obtenir le commit sélectionné
    let commit_oid = if let Some(commit) = state.selected_commit() {
        commit.oid
    } else {
        state.set_flash_message(flash_error_message("aucun commit sélectionné"));
        return Ok(());
    };

    let mut blame_state = BlameState::new(file_path.clone(), commit_oid);

    match crate::git::blame::blame_file(&state.repo.repo, commit_oid, &file_path) {
        Ok(blame) => {
            blame_state.blame = Some(blame);
            state.open_blame(blame_state);
        }
        Err(e) => {
            state.set_flash_message(flash_error("lors du blame", e));
        }
    }

    Ok(())
}

fn handle_close_blame(state: &mut AppState) -> Result<()> {
    if matches!(state.view_mode, ViewMode::Blame) {
        state.close_blame();
    }
    Ok(())
}

fn handle_jump_to_blame_commit(state: &mut AppState) -> Result<()> {
    if !matches!(state.view_mode, ViewMode::Blame) {
        return Ok(());
    }

    if let Some(ref blame_state) = state.blame_state {
        if let Some(ref blame) = blame_state.blame {
            if let Some(line) = blame.lines.get(blame_state.selected_line) {
                let target_oid = line.commit_oid;

                // Retour à la vue graph
                state.close_blame();

                // Chercher le commit dans le graphe en utilisant l'API unifiée
                if let Some(index) = state
                    .graph_view
                    .rows
                    .items()
                    .iter()
                    .position(|row| row.node.oid == target_oid)
                {
                    state.graph_view.select_commit(index);
                    let commit_short_id = format!("{:.7}", target_oid);
                    state.set_flash_message(format!("Sauté au commit {}", commit_short_id));
                } else {
                    state.set_flash_message(flash_error_message(
                        "commit non trouvé dans le graphe visible",
                    ));
                }
            }
        }
    }

    Ok(())
}

fn handle_commit_prompt(state: &mut AppState) -> Result<()> {
    // Basculer en vue Staging avec le focus sur le message de commit
    state.enter_view(ViewMode::Staging);
    state.staging_state.is_committing = true;
    state.staging_state.focus = StagingFocus::CommitMessage;
    state.staging_state.commit_message.clear();
    state.staging_state.cursor_position = 0;
    state.mark_dirty();
    // Charger le diff du premier fichier sélectionné
    crate::handler::staging::load_staging_diff(state);
    Ok(())
}

fn handle_stash_prompt(state: &mut AppState) -> Result<()> {
    // Créer un stash rapide sans message (WIP par défaut)
    match crate::git::stash::save_stash(&mut state.repo.repo, None) {
        Ok(_) => {
            state.set_flash_message(flash_success("Stash créé"));
            state.mark_dirty();
        }
        Err(e) => {
            state.set_flash_message(flash_error("stash", e));
        }
    }
    Ok(())
}

fn handle_merge_prompt(state: &mut AppState) -> Result<()> {
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
                state.set_flash_message(flash_error_message(
                    "aucune autre branche disponible pour merge",
                ));
                return Ok(());
            }

            state.merge_picker = Some(crate::state::MergePickerState::new(branch_names));
        }
        Err(e) => {
            state.set_flash_message(flash_error("chargement branches", e));
        }
    }
    Ok(())
}

fn handle_reset_prompt(state: &mut AppState) -> Result<()> {
    if !matches!(state.view_mode, ViewMode::Graph) {
        return Ok(());
    }

    // Utiliser l'API unifiée pour récupérer le commit sélectionné
    if let Some(commit) = state.selected_commit() {
        let oid = commit.oid;
        let short_hash = commit.oid.to_string();
        let short_hash = format!("{:.7}", short_hash);
        let commit_message = commit.message.lines().next().unwrap_or("").to_string();

        // Créer le reset picker
        state.reset_picker = Some(crate::state::ResetPickerState::new(
            oid,
            short_hash,
            commit_message,
        ));
    } else {
        state.set_flash_message(flash_error_message("aucun commit sélectionné"));
    }

    Ok(())
}

fn handle_abort_merge(state: &mut AppState) -> Result<()> {
    if !state.ui.is_merging {
        state.set_flash_message(flash_error_message("aucun merge en cours"));
        return Ok(());
    }

    // Demander confirmation via le dialogue existant
    state.open_confirmation(crate::ui::confirm_dialog::ConfirmAction::AbortMerge);

    Ok(())
}
