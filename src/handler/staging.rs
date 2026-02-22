//! Handler pour les actions de staging.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::StagingAction;
use crate::state::cache::DiffCacheKey;
use crate::state::{AppState, StagingFocus, ViewMode};

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
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Staging {
        if let Some(file) = state
            .staging_state
            .unstaged_files()
            .get(state.staging_state.unstaged_selected())
        {
            let path = file.path.clone();
            state.pending_confirmation = Some(ConfirmAction::DiscardFile(path));
        }
    }
    Ok(())
}

fn handle_discard_all(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Staging {
        state.pending_confirmation = Some(ConfirmAction::DiscardAll);
    }
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
    if state.view_mode == ViewMode::Staging && !state.staging_state.commit_message.is_empty() {
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
            .collect(),
    );

    state.staging_state.set_unstaged_files(
        all_entries
            .iter()
            .filter(|e| e.is_unstaged())
            .cloned()
            .collect(),
    );

    // Réajuster les sélections
    if state.staging_state.unstaged_selected() >= state.staging_state.unstaged_files().len() {
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
        // Pour le working directory, on utilise DiffCacheKey::working_dir()
        let cache_key = DiffCacheKey::working_dir(&file.path);

        // Essayer de récupérer du cache
        if let Some(cached_diff) = state.diff_cache.get(&cache_key) {
            state.staging_state.current_diff = Some(cached_diff.clone());
        } else {
            // Calculer et mettre en cache
            match crate::git::diff::working_dir_file_diff(&state.repo.repo, &file.path) {
                Ok(diff) => {
                    state.diff_cache.put(cache_key, diff.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::git::repo::StatusEntry;
    use tempfile::TempDir;

    /// Setup un repo temporaire pour les tests.
    fn setup_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(dir.path(), &opts).unwrap();

        // Configurer git
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        // Commit initial
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let git_repo = GitRepo::open(dir.path().to_str().unwrap()).unwrap();
        (dir, git_repo)
    }

    /// Crée un fichier dans le repo.
    fn create_test_file(dir: &TempDir, path: &str, content: &str) {
        let full_path = dir.path().join(path);
        std::fs::write(&full_path, content).unwrap();
    }

    #[test]
    fn test_switch_focus_in_staging() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;
        state.staging_state.focus = StagingFocus::Unstaged;

        let mut handler = StagingHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, StagingAction::SwitchFocus)
                .unwrap();
        }
        assert_eq!(state.staging_state.focus, StagingFocus::Staged);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, StagingAction::SwitchFocus)
                .unwrap();
        }
        assert_eq!(state.staging_state.focus, StagingFocus::Diff);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, StagingAction::SwitchFocus)
                .unwrap();
        }
        assert_eq!(state.staging_state.focus, StagingFocus::Unstaged);
    }

    #[test]
    fn test_start_commit_message() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, StagingAction::StartCommitMessage)
            .unwrap();

        drop(ctx);
        assert!(state.staging_state.is_committing);
        assert_eq!(state.staging_state.focus, StagingFocus::CommitMessage);
    }

    #[test]
    fn test_cancel_commit() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;
        state.staging_state.is_committing = true;
        state.staging_state.commit_message = "Test message".to_string();
        state.staging_state.focus = StagingFocus::CommitMessage;

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, StagingAction::CancelCommit)
            .unwrap();

        drop(ctx);
        assert!(!state.staging_state.is_committing);
        assert!(state.staging_state.commit_message.is_empty());
        assert_eq!(state.staging_state.focus, StagingFocus::Unstaged);
    }

    #[test]
    fn test_confirm_commit_with_empty_message_does_nothing() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;
        state.staging_state.is_committing = true;
        state.staging_state.commit_message = "".to_string();

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, StagingAction::ConfirmCommit)
            .unwrap();

        drop(ctx);
        // L'état ne devrait pas changer car le message est vide
        assert!(state.staging_state.is_committing);
    }

    #[test]
    fn test_stage_all_moves_files_to_staged() {
        let (dir, repo) = setup_test_repo();

        // Créer des fichiers non stagés
        create_test_file(&dir, "file1.txt", "content1");
        create_test_file(&dir, "file2.txt", "content2");

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        // Rafraîchir pour voir les fichiers non stagés
        refresh_staging(&mut state).unwrap();
        let unstaged_count = state.staging_state.unstaged_files().len();
        assert!(unstaged_count >= 2, "Devrait avoir des fichiers non stagés");

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, StagingAction::StageAll).unwrap();

        refresh_staging(&mut state).unwrap();

        // Tous les fichiers devraient être stagés
        assert_eq!(state.staging_state.unstaged_files().len(), 0);
        assert!(state.staging_state.staged_files().len() >= 2);
    }

    #[test]
    fn test_unstage_all_moves_files_to_unstaged() {
        let (dir, repo) = setup_test_repo();

        // Créer et stager des fichiers
        create_test_file(&dir, "file1.txt", "content1");
        let repo_ref = &repo.repo;
        let mut index = repo_ref.index().unwrap();
        index.add_path(std::path::Path::new("file1.txt")).unwrap();
        index.write().unwrap();

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        refresh_staging(&mut state).unwrap();
        assert!(
            !state.staging_state.staged_files().is_empty(),
            "Devrait avoir des fichiers stagés"
        );

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, StagingAction::UnstageAll).unwrap();

        refresh_staging(&mut state).unwrap();

        // Les fichiers devraient être non stagés
        assert_eq!(state.staging_state.staged_files().len(), 0);
        assert!(!state.staging_state.unstaged_files().is_empty());
    }

    #[test]
    fn test_stage_file_moves_single_file() {
        let (dir, repo) = setup_test_repo();

        // Créer des fichiers non stagés
        create_test_file(&dir, "file1.txt", "content1");
        create_test_file(&dir, "file2.txt", "content2");

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        refresh_staging(&mut state).unwrap();
        let initial_unstaged = state.staging_state.unstaged_files().len();
        assert!(initial_unstaged >= 2);

        // Sélectionner le premier fichier
        state.staging_state.set_unstaged_selected(0);

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, StagingAction::StageFile).unwrap();

        refresh_staging(&mut state).unwrap();

        // Un fichier de moins en unstaged
        assert_eq!(
            state.staging_state.unstaged_files().len(),
            initial_unstaged - 1
        );
        assert_eq!(state.staging_state.staged_files().len(), 1);
    }

    #[test]
    fn test_unstage_file_moves_single_file() {
        let (dir, repo) = setup_test_repo();

        // Créer et stager un fichier
        create_test_file(&dir, "file1.txt", "content1");
        let repo_ref = &repo.repo;
        let mut index = repo_ref.index().unwrap();
        index.add_path(std::path::Path::new("file1.txt")).unwrap();
        index.write().unwrap();

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        refresh_staging(&mut state).unwrap();
        assert_eq!(state.staging_state.staged_files().len(), 1);

        // Sélectionner le premier fichier stagé
        state.staging_state.set_staged_selected(0);

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, StagingAction::UnstageFile)
            .unwrap();

        refresh_staging(&mut state).unwrap();

        // Le fichier devrait être non stagé
        assert_eq!(state.staging_state.staged_files().len(), 0);
        assert_eq!(state.staging_state.unstaged_files().len(), 1);
    }

    #[test]
    fn test_confirm_commit_creates_commit() {
        let (dir, repo) = setup_test_repo();

        // Créer et stager un fichier
        create_test_file(&dir, "new_file.txt", "new content");
        let repo_ref = &repo.repo;
        let mut index = repo_ref.index().unwrap();
        index
            .add_path(std::path::Path::new("new_file.txt"))
            .unwrap();
        index.write().unwrap();

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;
        state.staging_state.is_committing = true;
        state.staging_state.commit_message = "Test commit".to_string();

        // Rafraîchir pour voir les fichiers stagés
        refresh_staging(&mut state).unwrap();
        assert!(!state.staging_state.staged_files().is_empty());

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, StagingAction::ConfirmCommit)
            .unwrap();

        // Vérifier que le commit a été créé
        let repo_ref = &state.repo.repo;
        let head = repo_ref.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert!(commit.message().unwrap().contains("Test commit"));

        // L'état devrait être réinitialisé
        assert!(!state.staging_state.is_committing);
        assert!(state.staging_state.commit_message.is_empty());
    }
}
