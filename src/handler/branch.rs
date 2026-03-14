//! Handler pour les actions sur les branches.

mod branches;
mod focus;
mod input;
mod stashes;
mod worktrees;

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::BranchAction;

use self::branches::{
    handle_checkout, handle_delete, handle_merge, handle_rename, handle_toggle_remote,
};
use self::focus::{
    handle_focus_detail, handle_focus_list, handle_select_local_branch,
    handle_select_remote_branch, handle_select_stash, handle_select_worktree,
};
use self::input::{
    handle_cancel_input, handle_confirm_input, handle_create, handle_next_section,
    handle_prev_section, handle_stash_save, handle_worktree_create,
};
pub use self::stashes::load_stash_file_diff;
use self::stashes::{
    handle_stash_apply, handle_stash_drop, handle_stash_file_next, handle_stash_file_prev,
    handle_stash_pop,
};
use self::worktrees::handle_worktree_remove;

/// Handler pour les opérations sur les branches.
pub struct BranchHandler;

impl ActionHandler for BranchHandler {
    type Action = BranchAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: BranchAction) -> Result<()> {
        match action {
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
            BranchAction::StashFileNext => handle_stash_file_next(ctx.state),
            BranchAction::StashFilePrev => handle_stash_file_prev(ctx.state),
            BranchAction::WorktreeCreate => handle_worktree_create(ctx.state),
            BranchAction::WorktreeRemove => handle_worktree_remove(ctx.state),
            BranchAction::NextSection => handle_next_section(ctx.state),
            BranchAction::PrevSection => handle_prev_section(ctx.state),
            BranchAction::ConfirmInput => handle_confirm_input(ctx.state),
            BranchAction::CancelInput => handle_cancel_input(ctx.state),
            BranchAction::SelectLocalBranch(index) => handle_select_local_branch(ctx.state, index),
            BranchAction::SelectRemoteBranch(index) => {
                handle_select_remote_branch(ctx.state, index)
            }
            BranchAction::SelectWorktree(index) => handle_select_worktree(ctx.state, index),
            BranchAction::SelectStash(index) => handle_select_stash(ctx.state, index),
            BranchAction::FocusList => handle_focus_list(ctx.state),
            BranchAction::FocusDetail => handle_focus_detail(ctx.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::state::{AppState, BranchesFocus, BranchesSection, InputAction, ViewMode};
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

    #[test]
    fn test_handle_next_section() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut handler = BranchHandler;

        state.branches_view_state.section = BranchesSection::Branches;
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::NextSection).unwrap();
        }
        assert_eq!(
            state.branches_view_state.section,
            BranchesSection::Worktrees
        );

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::NextSection).unwrap();
        }
        assert_eq!(state.branches_view_state.section, BranchesSection::Stashes);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::NextSection).unwrap();
        }
        assert_eq!(state.branches_view_state.section, BranchesSection::Branches);
    }

    #[test]
    fn test_handle_prev_section() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut handler = BranchHandler;

        state.branches_view_state.section = BranchesSection::Branches;
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::PrevSection).unwrap();
        }
        assert_eq!(state.branches_view_state.section, BranchesSection::Stashes);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::PrevSection).unwrap();
        }
        assert_eq!(
            state.branches_view_state.section,
            BranchesSection::Worktrees
        );

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::PrevSection).unwrap();
        }
        assert_eq!(state.branches_view_state.section, BranchesSection::Branches);
    }

    #[test]
    fn test_handle_toggle_remote() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        let mut handler = BranchHandler;

        assert!(!state.branches_view_state.show_remote);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, BranchAction::ToggleRemote)
                .unwrap();
        }
        assert!(state.branches_view_state.show_remote);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, BranchAction::ToggleRemote)
                .unwrap();
        }
        assert!(!state.branches_view_state.show_remote);
    }

    #[test]
    fn test_handle_create_branch() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        let mut handler = BranchHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::Create).unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::Input);
        assert_eq!(
            state.branches_view_state.input_action,
            Some(InputAction::CreateBranch)
        );
        assert!(state.branches_view_state.input_text.is_empty());
        assert_eq!(state.branches_view_state.input_cursor, 0);
    }

    #[test]
    fn test_handle_cancel_input() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateBranch);
        state.branches_view_state.input_text = "test-branch".to_string();
        state.branches_view_state.input_cursor = 5;
        let mut handler = BranchHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::CancelInput).unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::List);
        assert!(state.branches_view_state.input_action.is_none());
        assert!(state.branches_view_state.input_text.is_empty());
        assert_eq!(state.branches_view_state.input_cursor, 0);
    }

    #[test]
    fn test_handle_list_in_graph_view() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Graph;
    }

    #[test]
    fn test_handle_stash_save() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        let mut handler = BranchHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, BranchAction::StashSave).unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::Input);
        assert_eq!(
            state.branches_view_state.input_action,
            Some(InputAction::SaveStash)
        );
        assert!(state.branches_view_state.input_text.is_empty());
    }

    #[test]
    fn test_handle_worktree_create_opens_input() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        state.branches_view_state.section = BranchesSection::Worktrees;
        let mut handler = BranchHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, BranchAction::WorktreeCreate)
                .unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::Input);
        assert_eq!(
            state.branches_view_state.input_action,
            Some(InputAction::CreateWorktree)
        );
        assert!(state.branches_view_state.input_text.is_empty());
        assert_eq!(state.branches_view_state.input_cursor, 0);
    }

    #[test]
    fn test_handle_confirm_input_create_worktree_validation_empty() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateWorktree);
        state.branches_view_state.input_text = "".to_string();
        state.branches_view_state.input_cursor = 0;

        let mut handler = BranchHandler;
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, BranchAction::ConfirmInput)
                .unwrap();
        }

        assert_eq!(state.branches_view_state.focus, BranchesFocus::List);
        assert!(state.branches_view_state.input_action.is_none());
    }
}
