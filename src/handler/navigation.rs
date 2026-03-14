//! Handler pour les actions de navigation.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::NavigationAction;
use crate::state::{AppState, BranchesSection, FocusPanel, StagingFocus, ViewMode};

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
                load_commit_file_diff(ctx.state);
            }
            NavigationAction::FocusBottomRight => {
                ctx.state.focus = FocusPanel::BottomRight;
            }
            NavigationAction::SelectCommit(index) => {
                ctx.state.graph_view.select_commit(index);
                ctx.state.focus = FocusPanel::Graph;
                refresh_commit_file_data(ctx.state);
            }
            NavigationAction::SelectFile(index) => {
                ctx.state.graph_view.select_file(index);
                ctx.state.focus = FocusPanel::BottomLeft;
                load_commit_file_diff(ctx.state);
            }
        }

        Ok(())
    }
}

fn handle_move_up(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            if state.show_branch_panel {
                if state.branch_selected > 0 {
                    state.branch_selected -= 1;
                    state.dirty = true;
                }
            } else {
                state.graph_view.select_previous();
                // Charger les fichiers du nouveau commit sélectionné
                refresh_commit_file_data(state);
            }
        }
        ViewMode::Staging => {
            handle_staging_navigation(state, -1);
        }
        ViewMode::Branches => {
            handle_branches_navigation(state, -1);
        }
        ViewMode::Blame => {
            handle_blame_navigation(state, -1);
        }
        _ => {}
    }
}

fn handle_move_down(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            if state.show_branch_panel {
                if state.branch_selected + 1 < state.branches.len() {
                    state.branch_selected += 1;
                    state.dirty = true;
                }
            } else if !state.graph_view.is_empty() {
                state.graph_view.select_next();
                // Charger les fichiers du nouveau commit sélectionné
                refresh_commit_file_data(state);
                let _ = crate::handler::dispatcher::maybe_load_more_history(state);
            }
        }
        ViewMode::Staging => {
            handle_staging_navigation(state, 1);
        }
        ViewMode::Branches => {
            handle_branches_navigation(state, 1);
        }
        ViewMode::Blame => {
            handle_blame_navigation(state, 1);
        }
        _ => {}
    }
}

fn handle_page_up(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, -10);
            state.dirty = true;
        }
        _ => {
            if !state.show_branch_panel && !state.graph_view.is_empty() {
                state.graph_view.page_up();
                // Charger les fichiers du nouveau commit sélectionné
                refresh_commit_file_data(state);
            }
        }
    }
}

fn handle_page_down(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, 10);
            state.dirty = true;
        }
        _ => {
            if !state.show_branch_panel && !state.graph_view.is_empty() {
                state.graph_view.page_down();
                // Charger les fichiers du nouveau commit sélectionné
                refresh_commit_file_data(state);
                let _ = crate::handler::dispatcher::maybe_load_more_history(state);
            }
        }
    }
}

fn handle_go_top(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, -10000);
            state.dirty = true;
        }
        _ => {
            if !state.show_branch_panel {
                state.graph_view.go_top();
                // Charger les fichiers du nouveau commit sélectionné
                refresh_commit_file_data(state);
            }
        }
    }
}

fn handle_go_bottom(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, 10000);
            state.dirty = true;
        }
        _ => {
            if !state.show_branch_panel && !state.graph_view.is_empty() {
                let _ = crate::handler::dispatcher::load_all_history(state);
                state.graph_view.go_bottom();
                // Charger les fichiers du nouveau commit sélectionné
                refresh_commit_file_data(state);
            }
        }
    }
}

fn handle_switch_panel(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            state.focus = match state.focus {
                FocusPanel::Graph => FocusPanel::BottomLeft,
                FocusPanel::BottomLeft => FocusPanel::Graph,
                FocusPanel::BottomRight => FocusPanel::BottomLeft,
            };
            // Quand on passe au panneau BottomLeft, charger le diff du fichier sélectionné
            if state.focus == FocusPanel::BottomLeft {
                load_commit_file_diff(state);
            }
        }
        ViewMode::Staging => {
            state.staging_state.focus = match state.staging_state.focus {
                StagingFocus::Unstaged => StagingFocus::Staged,
                StagingFocus::Staged => StagingFocus::Diff,
                StagingFocus::Diff => StagingFocus::CommitMessage,
                StagingFocus::CommitMessage => StagingFocus::Unstaged,
            };
        }
        _ => {}
    }
}

/// Hauteur visible estimée du panneau diff (en lignes).
const DIFF_VISIBLE_HEIGHT_ESTIMATE: usize = 20;

fn handle_scroll_diff_up(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        if state.staging_state.diff_scroll > 0 {
            state.staging_state.diff_scroll -= 1;
        }
    } else {
        state.graph_view.scroll_diff_up();
    }
}

fn handle_scroll_diff_down(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.diff_scroll += 1;
    } else {
        state.graph_view.scroll_diff_down();
    }
}

fn handle_scroll_diff_page_up(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        let page_size = DIFF_VISIBLE_HEIGHT_ESTIMATE / 2;
        state.staging_state.diff_scroll = state.staging_state.diff_scroll.saturating_sub(page_size);
    } else {
        state.graph_view.scroll_diff_page_up();
    }
}

fn handle_scroll_diff_page_down(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        let page_size = DIFF_VISIBLE_HEIGHT_ESTIMATE / 2;
        state.staging_state.diff_scroll += page_size;
    } else {
        state.graph_view.scroll_diff_page_down();
    }
}

fn handle_scroll_diff_top(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.diff_scroll = 0;
    } else {
        state.graph_view.scroll_diff_top();
    }
}

fn handle_scroll_diff_bottom(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.diff_scroll = usize::MAX / 4;
    } else {
        state.graph_view.scroll_diff_bottom();
    }
}

fn handle_scroll_diff_left(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        if state.staging_state.diff_horizontal_offset > 0 {
            state.staging_state.diff_horizontal_offset -= 1;
        }
    } else {
        state.graph_view.scroll_diff_left();
    }
}

fn handle_scroll_diff_right(state: &mut AppState) {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.diff_horizontal_offset += 1;
    } else {
        state.graph_view.scroll_diff_right();
    }
}

fn handle_scroll_stash_diff_up(state: &mut AppState) {
    if state.branches_view_state.stash_diff_scroll > 0 {
        state.branches_view_state.stash_diff_scroll -= 1;
    }
}

fn handle_scroll_stash_diff_down(state: &mut AppState) {
    state.branches_view_state.stash_diff_scroll += 1;
}

fn handle_file_up(state: &mut AppState) {
    state.graph_view.select_previous_file();
    load_commit_file_diff(state);
}

fn handle_file_down(state: &mut AppState) {
    state.graph_view.select_next_file();
    load_commit_file_diff(state);
}

fn handle_back_to_graph(state: &mut AppState) {
    // Retourner au focus Graph (utilisé par Esc depuis BottomLeft/Files)
    if state.view_mode == ViewMode::Graph {
        state.focus = FocusPanel::Graph;
    }
}

/// Rafraîchit les données des fichiers du commit sélectionné.
/// Cette fonction est appelée après chaque changement de commit.
pub fn refresh_commit_file_data(state: &mut AppState) {
    state.refresh_commit_files();
    // Charger le diff du premier fichier si disponible
    if !state.graph_view.commit_files.is_empty() {
        load_commit_file_diff(state);
    } else {
        state.graph_view.clear_file_diff();
    }
}

fn handle_staging_navigation(state: &mut AppState, direction: i32) {
    match state.staging_state.focus {
        StagingFocus::Unstaged => {
            let max = state.staging_state.unstaged_files().len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.staging_state.unstaged_selected() + 1).min(max - 1)
                } else {
                    state.staging_state.unstaged_selected().saturating_sub(1)
                };
                state.staging_state.set_unstaged_selected(new_idx);
                // Recharger le diff après la navigation
                crate::handler::staging::load_staging_diff(state);
            }
        }
        StagingFocus::Staged => {
            let max = state.staging_state.staged_files().len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.staging_state.staged_selected() + 1).min(max - 1)
                } else {
                    state.staging_state.staged_selected().saturating_sub(1)
                };
                state.staging_state.set_staged_selected(new_idx);
                // Recharger le diff après la navigation
                crate::handler::staging::load_staging_diff(state);
            }
        }
        StagingFocus::Diff => {
            if direction > 0 {
                state.staging_state.diff_scroll += 1;
            } else if state.staging_state.diff_scroll > 0 {
                state.staging_state.diff_scroll -= 1;
            }
        }
        _ => {}
    }
}

fn handle_branches_navigation(state: &mut AppState, direction: i32) {
    match state.branches_view_state.section {
        BranchesSection::Branches => {
            let has_local = !state.branches_view_state.local_branches.is_empty();
            let has_remote = state.branches_view_state.show_remote
                && !state.branches_view_state.remote_branches.is_empty();

            if has_local || has_remote {
                if direction > 0 {
                    state.branches_view_state.select_next();
                } else {
                    state.branches_view_state.select_prev();
                }
            }
        }
        BranchesSection::Worktrees => {
            let max = state.branches_view_state.worktrees.len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.branches_view_state.worktree_selected() + 1).min(max - 1)
                } else {
                    state
                        .branches_view_state
                        .worktree_selected()
                        .saturating_sub(1)
                };
                state.branches_view_state.set_worktree_selected(new_idx);
            }
        }
        BranchesSection::Stashes => {
            let max = state.branches_view_state.stashes.len();
            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.branches_view_state.stash_selected() + 1).min(max - 1)
                } else {
                    state.branches_view_state.stash_selected().saturating_sub(1)
                };
                state.branches_view_state.set_stash_selected(new_idx);
                // Réinitialiser la sélection de fichier et charger le diff du premier fichier
                state.branches_view_state.stash_file_selected = 0;
                state.branches_view_state.stash_file_diff = None;
                state.branches_view_state.stash_diff_scroll = 0;
                // Charger le diff du premier fichier du nouveau stash sélectionné
                let _ = crate::handler::branch::load_stash_file_diff(state);
            }
        }
    }
}

fn handle_blame_navigation(state: &mut AppState, delta: i32) {
    if let Some(ref mut blame_state) = state.blame_state {
        let line_count = if let Some(ref blame) = blame_state.blame {
            blame.lines.len()
        } else {
            0
        };

        let new_idx = if delta >= 0 {
            (blame_state.selected_line + delta as usize).min(line_count.saturating_sub(1))
        } else {
            blame_state.selected_line.saturating_sub((-delta) as usize)
        };
        blame_state.selected_line = new_idx;
    }
}

/// Charge le diff pour le fichier sélectionné dans le commit courant.
pub fn load_commit_file_diff(state: &mut AppState) {
    if let Some(commit) = state.selected_commit() {
        let file_index = state.graph_view.file_selected_index;
        if let Some(file) = state.graph_view.commit_files.get(file_index) {
            let diff = state.repo.file_diff(commit.oid, &file.path).ok();
            state.graph_view.set_file_diff(diff);
            return;
        }
    }
    state.graph_view.clear_file_diff();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff::DiffStatus;
    use crate::git::graph::{CommitNode, GraphRow};
    use crate::git::repo::GitRepo;
    use crate::git::tests::test_utils::{commit, commit_file};
    use crate::state::selection::ListSelection;
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

    /// Helper pour créer un état de test avec un graph de taille donnée.
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
        // La taille de page est basée sur visible_height (10 par défaut)
        state.graph_view.rows.select(15);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::PageUp).unwrap();

        // page_up: 15 - 10 (visible_height) = 5
        assert_eq!(state.graph_view.selected_index(), 5);
    }

    #[test]
    fn test_page_down() {
        let mut state = create_test_state_with_graph(20);
        // La taille de page est basée sur visible_height (10 par défaut)
        state.graph_view.rows.select(5);

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::PageDown)
            .unwrap();

        // page_down: 5 + 10 (visible_height) = 15
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

        // Test FileDown
        {
            let mut ctx = HandlerContext { state: &mut state };
            handler
                .handle(&mut ctx, NavigationAction::FileDown)
                .unwrap();
        }
        assert_eq!(state.graph_view.file_selected_index, 1);

        // Test FileUp
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
