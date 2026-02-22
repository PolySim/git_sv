//! Handler pour les actions Git (remote, blame, cherry-pick, etc.).

use crate::error::Result;
use crate::state::{AppState, ViewMode, FocusPanel, BlameState};
use crate::state::action::GitAction;
use super::traits::{ActionHandler, HandlerContext};

/// Handler pour les opérations Git.
pub struct GitHandler;

impl ActionHandler for GitHandler {
    type Action = GitAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: GitAction) -> Result<()> {
        match action {
            GitAction::Push => handle_push(ctx.state),
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
            GitAction::BranchList => handle_branch_list(ctx.state),
        }
    }
}

fn handle_push(state: &mut AppState) -> Result<()> {
    match crate::git::remote::has_remote(&state.repo.repo) {
        Ok(true) => {
            match crate::git::remote::push_current_branch(&state.repo.repo) {
                Ok(msg) => {
                    state.set_flash_message(format!("{} ✓", msg));
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur lors du push: {}", e));
                }
            }
        }
        Ok(false) => {
            state.set_flash_message("Aucun remote configuré".to_string());
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur: {}", e));
        }
    }
    Ok(())
}

fn handle_pull(state: &mut AppState) -> Result<()> {
    use crate::git::conflict::MergeResult;
    use crate::state::ConflictsState;

    match crate::git::remote::has_remote(&state.repo.repo) {
        Ok(true) => {
            match crate::git::remote::pull_current_branch_with_result(&state.repo.repo) {
                Ok(MergeResult::UpToDate) => {
                    state.set_flash_message("Déjà à jour ✓".to_string());
                }
                Ok(MergeResult::FastForward) => {
                    state.set_flash_message("Pull (fast-forward) réussi ✓".to_string());
                    state.mark_dirty();
                }
                Ok(MergeResult::Success) => {
                    state.set_flash_message("Pull réussi ✓".to_string());
                    state.mark_dirty();
                }
                Ok(MergeResult::Conflicts(files)) => {
                    let ours_name = crate::git::conflict::get_current_branch_name(&state.repo.repo);
                    let theirs_name = format!(
                        "origin/{}",
                        state.current_branch.clone().unwrap_or_else(|| "HEAD".to_string())
                    );
                    state.conflicts_state = Some(ConflictsState::new(
                        files,
                        "Pull depuis origin".to_string(),
                        ours_name,
                        theirs_name,
                    ));
                    state.view_mode = ViewMode::Conflicts;
                    state.set_flash_message("Conflits lors du pull - résolution requise".to_string());
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur lors du pull: {}", e));
                }
            }
        }
        Ok(false) => {
            state.set_flash_message("Aucun remote configuré".to_string());
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur: {}", e));
        }
    }
    Ok(())
}

fn handle_fetch(state: &mut AppState) -> Result<()> {
    match crate::git::remote::has_remote(&state.repo.repo) {
        Ok(true) => {
            match crate::git::remote::fetch_all(&state.repo.repo) {
                Ok(_) => {
                    state.set_flash_message("Fetch réussi ✓".to_string());
                    state.mark_dirty();
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur lors du fetch: {}", e));
                }
            }
        }
        Ok(false) => {
            state.set_flash_message("Aucun remote configuré".to_string());
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur: {}", e));
        }
    }
    Ok(())
}

fn handle_cherry_pick(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if !matches!(state.view_mode, ViewMode::Graph) {
        return Ok(());
    }

    let commit_oid = if let Some(row) = state.graph.get(state.selected_index) {
        row.node.oid
    } else {
        state.set_flash_message("Aucun commit sélectionné".to_string());
        return Ok(());
    };

    state.pending_confirmation = Some(ConfirmAction::CherryPick(commit_oid));

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

    if !matches!(state.focus, FocusPanel::Files) {
        return Ok(());
    }

    if state.commit_files.is_empty() {
        state.set_flash_message("Aucun fichier sélectionné".to_string());
        return Ok(());
    }

    let selected_file = &state.commit_files[state.file_selected_index];
    let file_path = selected_file.path.clone();

    let commit_oid = if let Some(row) = state.graph.get(state.selected_index) {
        row.node.oid
    } else {
        state.set_flash_message("Aucun commit sélectionné".to_string());
        return Ok(());
    };

    let mut blame_state = BlameState::new(file_path.clone(), commit_oid);

    match crate::git::blame::blame_file(&state.repo.repo, commit_oid, &file_path) {
        Ok(blame) => {
            blame_state.blame = Some(blame);
            state.blame_state = Some(blame_state);
            state.view_mode = ViewMode::Blame;
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur lors du blame: {}", e));
        }
    }

    Ok(())
}

fn handle_close_blame(state: &mut AppState) -> Result<()> {
    if matches!(state.view_mode, ViewMode::Blame) {
        state.blame_state = None;
        state.view_mode = ViewMode::Graph;
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
                state.blame_state = None;
                state.view_mode = ViewMode::Graph;

                // Chercher le commit dans le graphe
                if let Some(index) = state.graph.iter().position(|row| row.node.oid == target_oid) {
                    state.selected_index = index;
                    state.graph_state.select(Some(index * 2));
                    state.sync_legacy_selection();
                    let commit_short_id = format!("{:.7}", target_oid);
                    state.set_flash_message(format!("Sauté au commit {}", commit_short_id));
                } else {
                    state.set_flash_message("Commit non trouvé dans le graphe visible".to_string());
                }
            }
        }
    }

    Ok(())
}

fn handle_commit_prompt(state: &mut AppState) -> Result<()> {
    // Ouvre le prompt de commit (affichage UI - pas d'opération directe)
    // L'UI s'occupera d'afficher le dialogue
    Ok(())
}

fn handle_stash_prompt(state: &mut AppState) -> Result<()> {
    // Ouvre le prompt de stash (affichage UI)
    Ok(())
}

fn handle_merge_prompt(state: &mut AppState) -> Result<()> {
    // Ouvre le sélecteur de branches pour merge
    // Cette fonction nécessite une logique UI
    Ok(())
}

fn handle_branch_list(state: &mut AppState) -> Result<()> {
    // Charge et affiche la liste des branches
    // Cette fonction nécessite une logique UI
    Ok(())
}
