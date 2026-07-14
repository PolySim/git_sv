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
            GitAction::RebasePrompt => handle_rebase_prompt(ctx.state),
            GitAction::ComparePrompt => handle_compare_prompt(ctx.state),
            GitAction::ClearComparison => handle_clear_comparison(ctx.state),
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
    state.staging_state.reset_commit_editing();
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
    state.staging_state.reset_commit_editing();
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
    open_branch_picker(
        state,
        crate::state::BranchPickerMode::Merge,
        "aucune autre branche disponible pour merge",
    );
    Ok(())
}

fn handle_rebase_prompt(state: &mut AppState) -> Result<()> {
    open_branch_picker(
        state,
        crate::state::BranchPickerMode::Rebase,
        "aucune autre branche disponible pour rebase",
    );
    Ok(())
}

fn handle_compare_prompt(state: &mut AppState) -> Result<()> {
    if state.view_mode != ViewMode::ProjectTree {
        return Ok(());
    }

    open_branch_picker(
        state,
        crate::state::BranchPickerMode::Compare,
        "aucune autre branche disponible pour comparaison",
    );
    Ok(())
}

fn open_branch_picker(
    state: &mut AppState,
    mode: crate::state::BranchPickerMode,
    empty_message: &str,
) {
    match crate::git::branch::list_all_branches(&state.repo.repo) {
        Ok((local, remote)) => {
            let current = state.current_branch.clone().unwrap_or_default();
            let mut branch_names = local
                .iter()
                .filter(|branch| branch.name != current)
                .map(|branch| branch.name.clone())
                .collect::<Vec<_>>();
            branch_names.extend(remote.into_iter().map(|branch| branch.name));

            if branch_names.is_empty() {
                state.set_flash_message(flash_error_message(empty_message));
                return;
            }

            state.merge_picker = Some(match mode {
                crate::state::BranchPickerMode::Merge => {
                    crate::state::MergePickerState::new(branch_names)
                }
                crate::state::BranchPickerMode::Rebase => {
                    crate::state::MergePickerState::new_rebase(branch_names)
                }
                crate::state::BranchPickerMode::Compare => {
                    crate::state::MergePickerState::new_compare(branch_names)
                }
            });
        }
        Err(error) => {
            state.set_flash_message(flash_error("chargement branches", error));
        }
    }
}

pub(crate) fn activate_project_tree_comparison(
    state: &mut AppState,
    target_branch: &str,
) -> Result<()> {
    let base_branch = match state.current_branch.clone() {
        Some(branch) => branch,
        None => {
            state.set_flash_message(flash_error_message(
                "comparaison impossible depuis une HEAD detachee",
            ));
            return Ok(());
        }
    };
    let Some(entry) = state.project_tree_state.selected_entry().cloned() else {
        state.set_flash_message(flash_error_message("aucun chemin sélectionné"));
        return Ok(());
    };

    let comparison = state.repo.compare_path_history(
        &entry.path,
        entry.is_directory(),
        target_branch,
        crate::state::MAX_TOTAL_COMMITS,
    )?;
    let ahead = comparison.ahead;
    let behind = comparison.behind;
    state
        .project_tree_state
        .start_comparison(base_branch.clone(), target_branch.to_string());
    state
        .project_tree_state
        .set_compared_path_history(comparison);
    state.project_tree_state.focus = crate::state::ProjectTreeFocus::History;
    state.set_flash_message(flash_success(format!(
        "Comparaison de '{}': {} ↔ {} · +{} / -{}",
        entry.path, base_branch, target_branch, ahead, behind
    )));

    Ok(())
}

fn handle_clear_comparison(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::ProjectTree {
        if state.project_tree_state.comparison.is_none() {
            return Ok(());
        }

        state.set_flash_message(flash_success("Comparaison du chemin fermee"));
        state.project_tree_state.clear_comparison();
        state.ensure_project_tree_focus_loaded();
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
