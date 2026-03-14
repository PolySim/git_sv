//! Handler pour les actions de navigation.

mod blame;
mod branches;
mod diff;
mod graph;

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::NavigationAction;
use crate::state::FocusPanel;

use self::blame::handle_blame_navigation;
use self::branches::handle_branches_navigation;
use self::diff::{
    handle_scroll_diff_bottom, handle_scroll_diff_down, handle_scroll_diff_left,
    handle_scroll_diff_page_down, handle_scroll_diff_page_up, handle_scroll_diff_right,
    handle_scroll_diff_top, handle_scroll_diff_up, handle_scroll_stash_diff_down,
    handle_scroll_stash_diff_up,
};
use self::graph::{
    handle_back_to_graph, handle_file_down, handle_file_up, handle_go_bottom, handle_go_top,
    handle_move_down, handle_move_up, handle_page_down, handle_page_up, handle_switch_panel,
};

/// Handler pour la navigation dans les listes.
pub struct NavigationHandler;

impl ActionHandler for NavigationHandler {
    type Action = NavigationAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: NavigationAction) -> Result<()> {
        match action {
            NavigationAction::MoveUp => handle_move_up(ctx.state),
            NavigationAction::MoveDown => handle_move_down(ctx.state),
            NavigationAction::PageUp => handle_page_up(ctx.state),
            NavigationAction::PageDown => handle_page_down(ctx.state),
            NavigationAction::GoTop => handle_go_top(ctx.state),
            NavigationAction::GoBottom => handle_go_bottom(ctx.state),
            NavigationAction::SwitchPanel => handle_switch_panel(ctx.state),
            NavigationAction::ScrollDiffUp => handle_scroll_diff_up(ctx.state),
            NavigationAction::ScrollDiffDown => handle_scroll_diff_down(ctx.state),
            NavigationAction::ScrollDiffPageUp => handle_scroll_diff_page_up(ctx.state),
            NavigationAction::ScrollDiffPageDown => handle_scroll_diff_page_down(ctx.state),
            NavigationAction::ScrollDiffTop => handle_scroll_diff_top(ctx.state),
            NavigationAction::ScrollDiffBottom => handle_scroll_diff_bottom(ctx.state),
            NavigationAction::ScrollDiffLeft => handle_scroll_diff_left(ctx.state),
            NavigationAction::ScrollDiffRight => handle_scroll_diff_right(ctx.state),
            NavigationAction::ScrollStashDiffUp => handle_scroll_stash_diff_up(ctx.state),
            NavigationAction::ScrollStashDiffDown => handle_scroll_stash_diff_down(ctx.state),
            NavigationAction::FileUp => handle_file_up(ctx.state),
            NavigationAction::FileDown => handle_file_down(ctx.state),
            NavigationAction::BackToGraph => handle_back_to_graph(ctx.state),
            NavigationAction::FocusGraph => ctx.state.focus = FocusPanel::Graph,
            NavigationAction::FocusBottomLeft => {
                ctx.state.focus = FocusPanel::BottomLeft;
                diff::load_commit_file_diff(ctx.state);
            }
            NavigationAction::FocusBottomRight => {
                ctx.state.focus = FocusPanel::BottomRight;
            }
            NavigationAction::SelectCommit(index) => {
                ctx.state.graph_view.select_commit(index);
                ctx.state.focus = FocusPanel::Graph;
                graph::refresh_commit_file_data(ctx.state);
            }
            NavigationAction::SelectFile(index) => {
                ctx.state.graph_view.select_file(index);
                ctx.state.focus = FocusPanel::BottomLeft;
                diff::load_commit_file_diff(ctx.state);
            }
        }

        Ok(())
    }
}

pub use diff::load_commit_file_diff;
pub use graph::refresh_commit_file_data;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff::DiffStatus;
    use crate::git::graph::{CommitNode, GraphRow};
    use crate::git::repo::GitRepo;
    use crate::git::tests::test_utils::{commit, commit_file};
    use crate::state::selection::ListSelection;
    use crate::state::AppState;
    use git2::Oid;
    use std::path::Path;

    fn create_test_graph(size: usize) -> Vec<GraphRow> {
        (0..size)
            .map(|i| GraphRow {
                node: CommitNode {
                    oid: Oid::from_bytes(&[i as u8; 20]).unwrap_or(Oid::zero()),
                    message: format!("Commit {} message", i),
                    author: "Test Author".to_string(),
                    timestamp: i as i64 * 1000,
                    parents: vec![],
                    refs: vec![],
                    branch_name: None,
                    column: 0,
                    color_index: 0,
                },
                cells: vec![None],
                connection: None,
            })
            .collect()
    }

    fn create_test_state_with_graph(size: usize) -> AppState {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let git_repo = GitRepo::open(temp_dir.path().to_str().unwrap()).unwrap();
        let mut state =
            AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();

        let graph = create_test_graph(size);
        state.graph_view.rows = ListSelection::with_items(graph);
        state.graph_view.rows.select(0);

        state
    }

    #[test]
    fn test_move_up_in_graph_view() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.rows.select(3);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::MoveUp).unwrap();

        assert_eq!(state.graph_view.selected_index(), 2);
    }

    #[test]
    fn test_move_up_at_top_stays_at_top() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.rows.select(0);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::MoveUp).unwrap();

        assert_eq!(state.graph_view.selected_index(), 0);
    }

    #[test]
    fn test_move_down_in_graph_view() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.rows.select(2);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::MoveDown)
            .unwrap();

        assert_eq!(state.graph_view.selected_index(), 3);
    }

    #[test]
    fn test_move_down_at_bottom_stays_at_bottom() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.rows.select(4);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::MoveDown)
            .unwrap();

        assert_eq!(state.graph_view.selected_index(), 4);
    }

    #[test]
    fn test_page_up() {
        let mut state = create_test_state_with_graph(20);
        state.graph_view.rows.select(15);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::PageUp).unwrap();

        assert_eq!(state.graph_view.selected_index(), 5);
    }

    #[test]
    fn test_page_down() {
        let mut state = create_test_state_with_graph(20);
        state.graph_view.rows.select(5);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::PageDown)
            .unwrap();

        assert_eq!(state.graph_view.selected_index(), 15);
    }

    #[test]
    fn test_go_top() {
        let mut state = create_test_state_with_graph(20);
        state.graph_view.rows.select(15);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::GoTop).unwrap();

        assert_eq!(state.graph_view.selected_index(), 0);
    }

    #[test]
    fn test_go_bottom() {
        let mut state = create_test_state_with_graph(20);
        state.graph_view.rows.select(5);
        state.graph_view.loaded_count = 20;
        state.graph_view.can_load_more = false;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::GoBottom)
            .unwrap();

        assert_eq!(state.graph_view.selected_index(), 19);
    }

    #[test]
    fn test_file_navigation() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.commit_files = vec![
            crate::git::diff::DiffFile {
                path: "a.txt".to_string(),
                old_path: None,
                status: DiffStatus::Added,
                additions: 1,
                deletions: 0,
            },
            crate::git::diff::DiffFile {
                path: "b.txt".to_string(),
                old_path: None,
                status: DiffStatus::Modified,
                additions: 0,
                deletions: 1,
            },
        ];
        state.graph_view.file_selected_index = 0;

        let mut handler = NavigationHandler;

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, NavigationAction::FileDown)
                .unwrap();
        }
        assert_eq!(state.graph_view.file_selected_index, 1);

        {
            let mut ctx = HandlerContext { state: &mut state };
            handler.handle(&mut ctx, NavigationAction::FileUp).unwrap();
        }
        assert_eq!(state.graph_view.file_selected_index, 0);
    }

    #[test]
    fn test_load_commit_file_diff_for_deleted_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        commit_file(&repo, "docs/deleted.txt", "hello", "Initial commit");
        std::fs::remove_file(temp_dir.path().join("docs/deleted.txt")).unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("docs/deleted.txt")).unwrap();
        index.write().unwrap();
        let deleted_commit_oid = commit(&repo, "Delete file");

        let git_repo = GitRepo::open(temp_dir.path().to_str().unwrap()).unwrap();
        let mut state =
            AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();

        state.graph_view.rows = ListSelection::with_items(vec![GraphRow {
            node: CommitNode {
                oid: deleted_commit_oid,
                message: "Delete file".to_string(),
                author: "Test".to_string(),
                timestamp: 0,
                parents: vec![],
                refs: vec![],
                branch_name: None,
                column: 0,
                color_index: 0,
            },
            cells: vec![None],
            connection: None,
        }]);
        state.graph_view.commit_files = state.repo.commit_diff(deleted_commit_oid).unwrap();
        state.graph_view.file_selected_index = 0;

        load_commit_file_diff(&mut state);

        let selected_diff = state
            .graph_view
            .selected_file_diff
            .as_ref()
            .expect("Le diff du fichier supprimé devrait être chargé");
        assert_eq!(selected_diff.path, "docs/deleted.txt");
        assert!(matches!(selected_diff.status, DiffStatus::Deleted));
    }
}
