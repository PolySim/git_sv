use super::*;
use crate::git::branch::BranchInfo;
use crate::git::repo::GitRepo;
use crate::state::action::{NavigationAction, SearchAction};
use crate::state::{BranchesSection, SelectedBranch};
use crate::test_utils::ui_driver::UiTestHarness;
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

#[test]
fn test_dispatch_quit_action() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    let mut dispatcher = ActionDispatcher::new();

    dispatcher.dispatch(&mut state, AppAction::Quit).unwrap();

    assert!(state.should_quit);
}

#[test]
fn test_dispatch_refresh_action() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    state.dirty = false;
    let mut dispatcher = ActionDispatcher::new();

    dispatcher.dispatch(&mut state, AppAction::Refresh).unwrap();

    assert!(state.dirty);
}

#[test]
fn test_dispatch_toggle_help() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    state.view_mode = ViewMode::Graph;
    let mut dispatcher = ActionDispatcher::new();

    // Activer l'aide
    dispatcher
        .dispatch(&mut state, AppAction::ToggleHelp)
        .unwrap();
    assert_eq!(state.view_mode, ViewMode::Help);
    assert_eq!(state.previous_view_mode, Some(ViewMode::Graph));

    // Désactiver l'aide
    dispatcher
        .dispatch(&mut state, AppAction::ToggleHelp)
        .unwrap();
    assert_eq!(state.view_mode, ViewMode::Graph);
    assert_eq!(state.previous_view_mode, None);
}

#[test]
fn test_dispatch_switch_view() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    state.view_mode = ViewMode::Graph;
    let mut dispatcher = ActionDispatcher::new();

    dispatcher
        .dispatch(&mut state, AppAction::SwitchView(ViewMode::Staging))
        .unwrap();

    assert_eq!(state.view_mode, ViewMode::Staging);
    assert!(state.dirty);
}

#[test]
fn test_dispatch_navigation_action() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    // Créer un graphe de test avec quelques commits
    state.graph_view.rows = crate::state::selection::ListSelection::with_items(
        (0..5)
            .map(|i| crate::git::graph::GraphRow {
                node: crate::git::graph::CommitNode {
                    oid: git2::Oid::from_bytes(&[i as u8; 20]).unwrap_or(git2::Oid::zero()),
                    message: format!("Commit {}", i),
                    author: "Test".to_string(),
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
            .collect(),
    );
    state.graph_view.rows.select(3);

    let mut dispatcher = ActionDispatcher::new();

    dispatcher
        .dispatch(&mut state, AppAction::Navigation(NavigationAction::GoTop))
        .unwrap();

    assert_eq!(state.graph_view.selected_index(), 0);
}

#[test]
fn test_dispatch_search_action() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    let mut dispatcher = ActionDispatcher::new();

    assert!(!state.search_state.is_active);

    dispatcher
        .dispatch(&mut state, AppAction::Search(SearchAction::Open))
        .unwrap();

    assert!(state.search_state.is_active);
}

#[test]
fn test_dispatch_confirm_action() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    use crate::ui::confirm_dialog::ConfirmAction;
    state.pending_confirmation = Some(ConfirmAction::DiscardAll);
    let mut dispatcher = ActionDispatcher::new();

    dispatcher
        .dispatch(&mut state, AppAction::ConfirmAction)
        .unwrap();

    // La confirmation devrait être consommée
    assert!(state.pending_confirmation.is_none());
}

#[test]
fn test_dispatch_cancel_action() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    use crate::ui::confirm_dialog::ConfirmAction;
    state.pending_confirmation = Some(ConfirmAction::DiscardAll);
    let mut dispatcher = ActionDispatcher::new();

    dispatcher
        .dispatch(&mut state, AppAction::CancelAction)
        .unwrap();

    assert!(state.pending_confirmation.is_none());
}

#[test]
fn test_dispatch_none_action() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    let initial_index = state.graph_view.selected_index();
    let mut dispatcher = ActionDispatcher::new();

    dispatcher.dispatch(&mut state, AppAction::None).unwrap();

    // Aucun changement d'état
    assert_eq!(state.graph_view.selected_index(), initial_index);
}

#[test]
fn test_dispatch_switch_bottom_mode() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    use crate::state::BottomLeftMode;
    state.bottom_left_mode = BottomLeftMode::Files;
    let mut dispatcher = ActionDispatcher::new();

    dispatcher
        .dispatch(&mut state, AppAction::SwitchBottomMode)
        .unwrap();

    assert_eq!(state.bottom_left_mode, BottomLeftMode::Parents);
}

#[test]
fn test_dispatch_select_from_bottom_left_opens_diff_panel() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    state.view_mode = ViewMode::Graph;
    state.focus = FocusPanel::BottomLeft;
    let mut dispatcher = ActionDispatcher::new();

    dispatcher.dispatch(&mut state, AppAction::Select).unwrap();

    assert_eq!(state.focus, FocusPanel::BottomRight);
    assert!(!state.graph_view.diff_fullscreen);
}

#[test]
fn test_dispatch_toggle_diff_fullscreen_restores_file_focus_when_closing() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    state.view_mode = ViewMode::Graph;
    state.focus = FocusPanel::BottomLeft;
    let mut dispatcher = ActionDispatcher::new();

    dispatcher
        .dispatch(&mut state, AppAction::ToggleDiffFullscreen)
        .unwrap();
    assert!(state.graph_view.diff_fullscreen);
    assert_eq!(state.focus, FocusPanel::BottomRight);

    dispatcher
        .dispatch(&mut state, AppAction::ToggleDiffFullscreen)
        .unwrap();
    assert!(!state.graph_view.diff_fullscreen);
    assert_eq!(state.focus, FocusPanel::BottomLeft);
}

#[test]
fn test_load_all_history_noop_when_no_more_history() {
    let (dir, repo) = setup_test_repo();
    let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string()).unwrap();
    state.graph_view.loaded_count = state.graph_view.len();
    state.graph_view.can_load_more = false;

    let loaded = super::load_all_history(&mut state).unwrap();

    assert!(!loaded);
    assert_eq!(state.graph_view.loaded_count, state.graph_view.len());
}

#[test]
fn test_ui_flow_branch_creation_from_keyboard_input() {
    let mut harness = UiTestHarness::new();

    harness.send_char('3');
    harness.send_char('n');
    harness.send_text("feature/ui-flow");
    harness.send_enter();

    assert_eq!(harness.state.view_mode, ViewMode::Branches);
    assert!(harness.state.branches_view_state.input_action.is_none());
    assert_eq!(
        harness.state.current_flash_message(),
        Some("Branche 'feature/ui-flow' créée ✓")
    );
    assert!(harness
        .state
        .repo
        .repo
        .find_branch("feature/ui-flow", git2::BranchType::Local)
        .is_ok());
}

#[test]
fn test_ui_flow_stash_save_from_branches_input() {
    let mut harness = UiTestHarness::new();
    harness.commit_file("tracked.txt", "base\n", "Add tracked");
    harness.write_file("tracked.txt", "base\nmodifie\n");
    harness.stage_file("tracked.txt");

    harness.send_char('3');
    harness.send_tab();
    harness.send_tab();
    harness.send_char('s');
    harness.send_text("stash ui");
    harness.send_enter();

    let mut stash_count = 0;
    harness
        .state
        .repo
        .repo
        .stash_foreach(|_, message, _| {
            stash_count += 1;
            assert!(message.contains("stash ui"));
            true
        })
        .unwrap();

    assert_eq!(stash_count, 1);
    assert_eq!(
        harness.state.current_flash_message(),
        Some("Stash créé: stash ui ✓")
    );
    assert_eq!(
        harness.state.branches_view_state.focus,
        crate::state::BranchesFocus::List
    );
}

#[test]
fn test_ui_flow_invalid_worktree_input_shows_validation_message() {
    let mut harness = UiTestHarness::new();

    harness.send_char('3');
    harness.send_tab();
    harness.send_char('n');
    harness.send_text("worktree-seul");
    harness.send_enter();

    assert_eq!(harness.state.view_mode, ViewMode::Branches);
    assert_eq!(
        harness.state.current_flash_message(),
        Some("Format: nom chemin [branche]")
    );
    assert!(harness.state.branches_view_state.input_action.is_none());
}

#[test]
fn test_ui_flow_discard_all_confirmation_roundtrip() {
    let mut harness = UiTestHarness::new();
    harness.commit_file("tracked.txt", "base\n", "Add tracked");
    harness.write_file("tracked.txt", "base\nmodifie\n");
    harness.refresh_staging();

    harness.send_char('2');
    harness.send_char('D');
    assert_eq!(
        harness.state.pending_confirmation,
        Some(crate::ui::confirm_dialog::ConfirmAction::DiscardAll)
    );

    harness.send_char('y');

    assert!(harness.state.pending_confirmation.is_none());
    assert_eq!(
        harness.state.current_flash_message(),
        Some("Modifications ignorées ✓")
    );
    let statuses = harness.state.repo.repo.statuses(None).unwrap();
    assert!(statuses.is_empty());
}

#[test]
fn test_ui_flow_graph_to_files_to_diff_via_keyboard() {
    let mut harness = UiTestHarness::new();
    harness.commit_file("file_a.txt", "alpha\n", "Add alpha");
    harness.commit_file("file_b.txt", "beta\n", "Add beta");
    harness.refresh_graph();

    harness.send_enter();
    assert_eq!(harness.state.focus, FocusPanel::BottomLeft);
    assert!(!harness.state.graph_view.commit_files.is_empty());
    assert!(harness.state.graph_view.selected_file_diff.is_some());

    harness.send_char(' ');
    assert_eq!(harness.state.focus, FocusPanel::BottomRight);
    assert!(!harness.state.graph_view.diff_fullscreen);
}

#[test]
fn test_ui_flow_search_then_filter_clears_search_state() {
    let mut harness = UiTestHarness::new();
    harness.commit_file("first.txt", "a\n", "Fix login bug");
    harness.commit_file("second.txt", "b\n", "Add search feature");
    harness.refresh_graph();

    harness.send_char('/');
    harness.send_text("search");

    assert!(harness.state.search_state.is_active);
    assert_eq!(harness.state.search_state.query, "search");
    assert!(!harness.state.search_state.results.is_empty());

    harness.send_esc();
    harness.send_char('F');
    for _ in 0..4 {
        harness.send_tab();
    }
    harness.send_text("feature");
    harness.send_enter();

    assert_eq!(
        harness.state.graph_filter.message.as_deref(),
        Some("feature")
    );
    assert!(!harness.state.search_state.is_active);
    assert!(harness.state.search_state.results.is_empty());
    assert!(harness.state.search_state.query.is_empty());
    assert_eq!(
        harness.state.current_flash_message(),
        Some("Filtres actifs: message")
    );
}

#[test]
fn test_ui_flow_branches_navigation_reaches_remote_selection() {
    let mut harness = UiTestHarness::new();
    harness.state.view_mode = ViewMode::Branches;
    harness.state.branches_view_state.section = BranchesSection::Branches;
    harness.state.branches_view_state.show_remote = true;
    harness
        .state
        .branches_view_state
        .local_branches
        .set_items(vec![
            BranchInfo::simple("main".to_string(), true, false),
            BranchInfo::simple("feature".to_string(), false, false),
        ]);
    harness
        .state
        .branches_view_state
        .remote_branches
        .set_items(vec![BranchInfo::simple(
            "origin/main".to_string(),
            false,
            true,
        )]);
    harness.state.branches_view_state.selected_branch = Some(SelectedBranch::Local(1));

    harness.send_char('j');

    assert_eq!(
        harness.state.branches_view_state.selected_branch,
        Some(SelectedBranch::Remote(0))
    );
}
