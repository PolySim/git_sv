//! Handler pour les actions de staging.

mod commit_ops;
mod file_ops;
mod focus;
mod refresh;
mod stash_ops;

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::StagingAction;

use self::commit_ops::{handle_cancel_commit, handle_confirm_commit, handle_start_commit};
use self::file_ops::{
    handle_discard_all, handle_discard_file, handle_stage_all, handle_stage_file,
    handle_stage_hunk, handle_stage_line, handle_unstage_all, handle_unstage_file,
    handle_unstage_hunk, handle_unstage_line,
};
use self::focus::{
    handle_focus_diff, handle_focus_staged, handle_focus_unstaged, handle_select_staged,
    handle_select_unstaged, handle_switch_focus,
};
use self::stash_ops::{handle_stash_selected_file, handle_stash_unstaged_files};

pub use self::refresh::{load_staging_diff, refresh_staging};

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
            StagingAction::StageHunk => handle_stage_hunk(ctx.state),
            StagingAction::UnstageHunk => handle_unstage_hunk(ctx.state),
            StagingAction::StageLine => handle_stage_line(ctx.state),
            StagingAction::UnstageLine => handle_unstage_line(ctx.state),
            StagingAction::DiscardFile => handle_discard_file(ctx.state),
            StagingAction::DiscardAll => handle_discard_all(ctx.state),
            StagingAction::StartCommitMessage => handle_start_commit(ctx.state),
            StagingAction::ConfirmCommit => handle_confirm_commit(ctx.state),
            StagingAction::CancelCommit => handle_cancel_commit(ctx.state),
            StagingAction::SwitchFocus => handle_switch_focus(ctx.state),
            StagingAction::FocusDiff => handle_focus_diff(ctx.state),
            StagingAction::StashSelectedFile => handle_stash_selected_file(ctx.state),
            StagingAction::StashUnstagedFiles => handle_stash_unstaged_files(ctx.state),
            StagingAction::FocusUnstaged => handle_focus_unstaged(ctx.state),
            StagingAction::FocusStaged => handle_focus_staged(ctx.state),
            StagingAction::SelectUnstaged(index) => handle_select_unstaged(ctx.state, index),
            StagingAction::SelectStaged(index) => handle_select_staged(ctx.state, index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::state::{AppState, StagingFocus, ViewMode};
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(dir.path(), &opts).unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let git_repo = GitRepo::open(dir.path().to_str().unwrap()).unwrap();
        (dir, git_repo)
    }

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
        assert_eq!(state.staging_state.focus, StagingFocus::Unstaged);
    }

    #[test]
    fn test_focus_diff_and_return_to_previous_list() {
        let (dir, repo) = setup_test_repo();
        create_test_file(&dir, "file.txt", "content");
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;
        state.staging_state.focus = StagingFocus::Unstaged;
        refresh_staging(&mut state).unwrap();

        let mut handler = StagingHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, StagingAction::FocusDiff).unwrap();
        }
        assert_eq!(state.staging_state.focus, StagingFocus::Diff);
        assert_eq!(state.staging_state.last_file_focus, StagingFocus::Unstaged);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, StagingAction::SwitchFocus)
                .unwrap();
        }
        assert_eq!(state.staging_state.focus, StagingFocus::Unstaged);
    }

    #[test]
    fn test_focus_diff_keeps_selected_file_diff_visible() {
        let (dir, repo) = setup_test_repo();
        create_test_file(&dir, "file.txt", "initial");
        {
            let git_repo = git2::Repository::open(dir.path()).unwrap();
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let sig = git2::Signature::now("Test", "test@test.com").unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_oid).unwrap();
            let head = git_repo.head().unwrap();
            let parent = git_repo.find_commit(head.target().unwrap()).unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "Add file", &tree, &[&parent])
                .unwrap();
        }
        create_test_file(&dir, "file.txt", "modified");
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;
        state.staging_state.focus = StagingFocus::Unstaged;
        refresh_staging(&mut state).unwrap();

        let mut handler = StagingHandler;
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, StagingAction::FocusDiff).unwrap();
        }

        assert_eq!(state.staging_state.focus, StagingFocus::Diff);
        assert!(state.staging_state.current_diff.is_some());
        assert_eq!(
            state.staging_state.current_diff.as_ref().unwrap().path,
            "file.txt"
        );
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
        assert!(state.staging_state.is_committing);
    }

    #[test]
    fn test_stage_all_moves_files_to_staged() {
        let (dir, repo) = setup_test_repo();
        create_test_file(&dir, "file1.txt", "content1");
        create_test_file(&dir, "file2.txt", "content2");

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        refresh_staging(&mut state).unwrap();
        let unstaged_count = state.staging_state.unstaged_files().len();
        assert!(unstaged_count >= 2);

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, StagingAction::StageAll).unwrap();

        refresh_staging(&mut state).unwrap();
        assert_eq!(state.staging_state.unstaged_files().len(), 0);
        assert!(state.staging_state.staged_files().len() >= 2);
    }

    #[test]
    fn test_unstage_all_moves_files_to_unstaged() {
        let (dir, repo) = setup_test_repo();
        create_test_file(&dir, "file1.txt", "content1");
        let repo_ref = &repo.repo;
        let mut index = repo_ref.index().unwrap();
        index.add_path(std::path::Path::new("file1.txt")).unwrap();
        index.write().unwrap();

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        refresh_staging(&mut state).unwrap();
        assert!(!state.staging_state.staged_files().is_empty());

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, StagingAction::UnstageAll).unwrap();

        refresh_staging(&mut state).unwrap();
        assert_eq!(state.staging_state.staged_files().len(), 0);
        assert!(!state.staging_state.unstaged_files().is_empty());
    }

    #[test]
    fn test_stage_file_moves_single_file() {
        let (dir, repo) = setup_test_repo();
        create_test_file(&dir, "file1.txt", "content1");
        create_test_file(&dir, "file2.txt", "content2");

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        refresh_staging(&mut state).unwrap();
        let initial_unstaged = state.staging_state.unstaged_files().len();
        assert!(initial_unstaged >= 2);

        state.staging_state.set_unstaged_selected(0);

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, StagingAction::StageFile).unwrap();

        refresh_staging(&mut state).unwrap();
        assert_eq!(
            state.staging_state.unstaged_files().len(),
            initial_unstaged - 1
        );
        assert_eq!(state.staging_state.staged_files().len(), 1);
    }

    #[test]
    fn test_unstage_file_moves_single_file() {
        let (dir, repo) = setup_test_repo();
        create_test_file(&dir, "file1.txt", "content1");
        let repo_ref = &repo.repo;
        let mut index = repo_ref.index().unwrap();
        index.add_path(std::path::Path::new("file1.txt")).unwrap();
        index.write().unwrap();

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;

        refresh_staging(&mut state).unwrap();
        assert_eq!(state.staging_state.staged_files().len(), 1);

        state.staging_state.set_staged_selected(0);

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, StagingAction::UnstageFile)
            .unwrap();

        refresh_staging(&mut state).unwrap();
        assert_eq!(state.staging_state.staged_files().len(), 0);
        assert_eq!(state.staging_state.unstaged_files().len(), 1);
    }

    #[test]
    fn test_confirm_commit_creates_commit() {
        let (dir, repo) = setup_test_repo();
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

        refresh_staging(&mut state).unwrap();
        assert!(!state.staging_state.staged_files().is_empty());

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, StagingAction::ConfirmCommit)
            .unwrap();

        let repo_ref = &state.repo.repo;
        let head = repo_ref.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert!(commit.message().unwrap().contains("Test commit"));
        assert!(!state.staging_state.is_committing);
        assert!(state.staging_state.commit_message.is_empty());
    }

    #[test]
    fn test_stash_selected_file_updates_flash_and_staging_state() {
        let (dir, repo) = setup_test_repo();
        create_test_file(&dir, "tracked.txt", "base\n");
        {
            let repo_ref = &repo.repo;
            let mut index = repo_ref.index().unwrap();
            index.add_path(std::path::Path::new("tracked.txt")).unwrap();
            index.write().unwrap();
            let sig = git2::Signature::now("Test", "test@test.com").unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo_ref.find_tree(tree_oid).unwrap();
            let head = repo_ref.head().unwrap();
            let parent = repo_ref.find_commit(head.target().unwrap()).unwrap();
            repo_ref
                .commit(Some("HEAD"), &sig, &sig, "Add tracked", &tree, &[&parent])
                .unwrap();
        }

        create_test_file(&dir, "tracked.txt", "base\nstaged\n");
        {
            let repo_ref = &repo.repo;
            let mut index = repo_ref.index().unwrap();
            index.add_path(std::path::Path::new("tracked.txt")).unwrap();
            index.write().unwrap();
        }
        create_test_file(&dir, "tracked.txt", "base\nstaged\nunstaged\n");

        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Staging;
        refresh_staging(&mut state).unwrap();

        let mut handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };
        handler
            .handle(&mut ctx, StagingAction::StashSelectedFile)
            .unwrap();

        drop(ctx);
        assert_eq!(state.staging_state.unstaged_files().len(), 0);
        assert_eq!(state.staging_state.staged_files().len(), 1);
        assert_eq!(
            state.current_flash_message(),
            Some("Stash créé pour tracked.txt (index conservé) ✓")
        );
    }
}
