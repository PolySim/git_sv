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
            NavigationAction::FileUp => handle_file_up(ctx.state),
            NavigationAction::FileDown => handle_file_down(ctx.state),
            NavigationAction::BackToGraph => handle_back_to_graph(ctx.state),
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
                }
            } else if state.selected_index > 0 {
                let new_index = state.selected_index - 1;
                state.graph_view.rows.select(new_index);
                state.sync_graph_selection();
                state.sync_legacy_selection();
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
                }
            } else if state.selected_index + 1 < state.graph.len() {
                let new_index = state.selected_index + 1;
                state.graph_view.rows.select(new_index);
                state.sync_graph_selection();
                state.sync_legacy_selection();
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
        }
        _ => {
            if !state.show_branch_panel && !state.graph.is_empty() {
                let page_size = 10;
                let new_index = state.selected_index.saturating_sub(page_size);
                state.graph_view.rows.select(new_index);
                state.sync_graph_selection();
                state.sync_legacy_selection();
            }
        }
    }
}

fn handle_page_down(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, 10);
        }
        _ => {
            if !state.show_branch_panel && !state.graph.is_empty() {
                let page_size = 10;
                let new_index = (state.selected_index + page_size).min(state.graph.len() - 1);
                state.graph_view.rows.select(new_index);
                state.sync_graph_selection();
                state.sync_legacy_selection();
            }
        }
    }
}

fn handle_go_top(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, -10000);
        }
        _ => {
            if !state.show_branch_panel {
                state.graph_view.rows.select(0);
                state.sync_graph_selection();
                state.sync_legacy_selection();
            }
        }
    }
}

fn handle_go_bottom(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Blame => {
            handle_blame_navigation(state, 10000);
        }
        _ => {
            if !state.show_branch_panel && !state.graph.is_empty() {
                let new_index = state.graph.len() - 1;
                state.graph_view.rows.select(new_index);
                state.sync_graph_selection();
                state.sync_legacy_selection();
            }
        }
    }
}

fn handle_switch_panel(state: &mut AppState) {
    match state.view_mode {
        ViewMode::Graph => {
            state.focus = match state.focus {
                FocusPanel::Graph => FocusPanel::BottomLeft,
                FocusPanel::BottomLeft => FocusPanel::BottomRight,
                FocusPanel::BottomRight => FocusPanel::Graph,
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

fn handle_scroll_diff_up(state: &mut AppState) {
    if state.diff_scroll_offset > 0 {
        state.diff_scroll_offset -= 1;
    }
}

fn handle_scroll_diff_down(state: &mut AppState) {
    state.diff_scroll_offset += 1;
}

fn handle_file_up(state: &mut AppState) {
    if state.file_selected_index > 0 {
        state.file_selected_index -= 1;
        state.graph_view.file_selected_index = state.file_selected_index;
        // Charger le diff du fichier sélectionné
        load_commit_file_diff(state);
    }
}

fn handle_file_down(state: &mut AppState) {
    if state.file_selected_index + 1 < state.commit_files.len() {
        state.file_selected_index += 1;
        state.graph_view.file_selected_index = state.file_selected_index;
        // Charger le diff du fichier sélectionné
        load_commit_file_diff(state);
    }
}

fn handle_back_to_graph(state: &mut AppState) {
    // Retourner au focus Graph (utilisé par Esc depuis BottomLeft/Files)
    if state.view_mode == ViewMode::Graph {
        state.focus = FocusPanel::Graph;
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
            let local_count = state.branches_view_state.local_branches.len();
            let remote_count = state.branches_view_state.remote_branches.len();
            let show_remote = state.branches_view_state.show_remote;

            let max = if show_remote && remote_count > 0 {
                local_count + remote_count
            } else {
                local_count
            };

            if max > 0 {
                let new_idx = if direction > 0 {
                    (state.branches_view_state.branch_selected() + 1).min(max - 1)
                } else {
                    state
                        .branches_view_state
                        .branch_selected()
                        .saturating_sub(1)
                };
                state.branches_view_state.set_branch_selected(new_idx);
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
    if let Some(row) = state.graph.get(state.selected_index) {
        if let Some(file) = state.commit_files.get(state.file_selected_index) {
            state.selected_file_diff = state.repo.file_diff(row.node.oid, &file.path).ok();
            state.graph_view.diff_scroll_offset = 0;
            return;
        }
    }
    state.selected_file_diff = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{CommitNode, GraphRow};
    use crate::git::repo::GitRepo;
    use crate::state::selection::ListSelection;
    use git2::Oid;

    /// Helper pour créer un état de test avec un graph de taille donnée.
    fn create_test_state_with_graph(size: usize) -> AppState {
        // Créer un repo temporaire
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();

        // Configurer git
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        // Créer un commit initial
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let git_repo = GitRepo::open(temp_dir.path().to_str().unwrap()).unwrap();
        let mut state =
            AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();

        // Créer un graph de test
        let graph: Vec<GraphRow> = (0..size)
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
            .collect();

        state.graph = graph;
        state.graph_view.rows = ListSelection::with_items(state.graph.clone());
        state.graph_view.rows.select(0);
        state.selected_index = 0;
        state.graph_state.select(Some(0));

        state
    }

    #[test]
    fn test_move_up_in_graph_view() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.rows.select(3); // Position initiale
        state.selected_index = 3;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::MoveUp).unwrap();

        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_move_up_at_top_stays_at_top() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.rows.select(0);
        state.selected_index = 0;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::MoveUp).unwrap();

        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_move_down_in_graph_view() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.rows.select(2);
        state.selected_index = 2;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::MoveDown)
            .unwrap();

        assert_eq!(state.selected_index, 3);
    }

    #[test]
    fn test_move_down_at_bottom_stays_at_bottom() {
        let mut state = create_test_state_with_graph(5);
        state.graph_view.rows.select(4); // Dernier élément
        state.selected_index = 4;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::MoveDown)
            .unwrap();

        assert_eq!(state.selected_index, 4);
    }

    #[test]
    fn test_page_up() {
        let mut state = create_test_state_with_graph(20);
        state.graph_view.rows.set_visible_height(5);
        state.graph_view.rows.select(15);
        state.selected_index = 15;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::PageUp).unwrap();

        assert_eq!(state.selected_index, 5); // 15 - 10 = 5
    }

    #[test]
    fn test_page_down() {
        let mut state = create_test_state_with_graph(20);
        state.graph_view.rows.set_visible_height(5);
        state.graph_view.rows.select(5);
        state.selected_index = 5;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::PageDown)
            .unwrap();

        assert_eq!(state.selected_index, 15); // 5 + 10 = 15
    }

    #[test]
    fn test_go_top() {
        let mut state = create_test_state_with_graph(20);
        state.graph_view.rows.select(15);
        state.selected_index = 15;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler.handle(&mut ctx, NavigationAction::GoTop).unwrap();

        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_go_bottom() {
        let mut state = create_test_state_with_graph(20);
        state.graph_view.rows.select(5);
        state.selected_index = 5;

        let mut handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };

        handler
            .handle(&mut ctx, NavigationAction::GoBottom)
            .unwrap();

        assert_eq!(state.selected_index, 19);
    }
}
