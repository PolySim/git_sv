use super::keyboard::map_key;
use super::mouse::map_mouse;
use crate::git::repo::GitRepo;
use crate::git::tests::test_utils::create_test_repo;
use crate::state::action::{
    FilterAction, GitAction, NavigationAction, SearchAction, StagingAction,
};
use crate::state::{AppAction, AppState, FocusPanel, ResetPickerState, StagingFocus, ViewMode};
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

fn create_test_state() -> AppState {
    let (temp_dir, _repo) = create_test_repo();
    let git_repo = GitRepo::open(temp_dir.path().to_string_lossy().as_ref()).unwrap();
    let mut state = AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();
    state.screen_area = Rect::new(0, 0, 120, 40);
    state
}

#[test]
fn test_search_mode_arrow_down_moves_to_next_result() {
    let mut state = create_test_state();
    state.search_state.is_active = true;

    let action = map_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &state);

    assert_eq!(action, Some(AppAction::Search(SearchAction::NextResult)));
}

#[test]
fn test_search_mode_ctrl_n_moves_to_next_result() {
    let mut state = create_test_state();
    state.search_state.is_active = true;

    let action = map_key(
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
        &state,
    );

    assert_eq!(action, Some(AppAction::Search(SearchAction::NextResult)));
}

#[test]
fn test_filter_popup_arrow_down_moves_to_next_field() {
    let mut state = create_test_state();
    state.filters.filter_popup.is_open = true;

    let action = map_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &state);

    assert_eq!(action, Some(AppAction::Filter(FilterAction::NextField)));
}

#[test]
fn test_help_mode_esc_closes_help() {
    let mut state = create_test_state();
    state.toggle_help();

    let action = map_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &state);

    assert_eq!(action, Some(AppAction::ToggleHelp));
}

#[test]
fn test_branches_tab_switches_section() {
    let mut state = create_test_state();
    state.view_mode = ViewMode::Branches;

    let action = map_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), &state);

    assert_eq!(
        action,
        Some(AppAction::Branch(
            crate::state::action::BranchAction::NextSection
        ))
    );
}

#[test]
fn test_graph_question_mark_opens_help() {
    let state = create_test_state();

    let action = map_key(
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::ToggleHelp));
}

#[test]
fn test_graph_history_safety_shortcuts() {
    let state = create_test_state();

    let interactive_rebase = map_key(
        KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT),
        &state,
    );
    let undo_reflog = map_key(
        KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::SHIFT),
        &state,
    );

    assert_eq!(
        interactive_rebase,
        Some(AppAction::Git(GitAction::InteractiveRebase))
    );
    assert_eq!(
        undo_reflog,
        Some(AppAction::Git(GitAction::UndoLastOperation))
    );
}

#[test]
fn test_project_tree_c_opens_branch_comparison_picker() {
    let mut state = create_test_state();
    state.view_mode = ViewMode::ProjectTree;

    let action = map_key(
        KeyEvent::new(KeyCode::Char('C'), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::Git(GitAction::ComparePrompt)));
}

#[test]
fn test_graph_reference_shortcuts() {
    let state = create_test_state();

    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Git(GitAction::CreateTag))
    );
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT),
            &state
        ),
        Some(AppAction::Git(GitAction::DeleteTag))
    );
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT),
            &state
        ),
        Some(AppAction::Git(GitAction::CompareSelectedWithHead))
    );
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT),
            &state
        ),
        Some(AppAction::Git(GitAction::BisectStart))
    );
}

#[test]
fn test_graph_bisect_shortcuts_are_active_during_bisect() {
    let mut state = create_test_state();
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
            &state
        ),
        None
    );

    state.ui.is_bisecting = true;
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Git(GitAction::BisectGood))
    );
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Git(GitAction::BisectBad))
    );
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::NONE),
            &state,
        ),
        Some(AppAction::Git(GitAction::BisectReset))
    );
}

#[test]
fn test_graph_i_opens_and_closes_repository_insights() {
    let mut state = create_test_state();
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Git(GitAction::RepositoryInsights))
    );

    state.ui.repository_insights = Some(crate::git::insights::RepositoryInsights {
        commit: git2::Oid::zero().to_string(),
        signature: crate::git::insights::CommitSignatureStatus::Unsigned,
        hooks: Vec::new(),
        submodules: Vec::new(),
    });
    assert_eq!(
        map_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &state),
        Some(AppAction::Git(GitAction::RepositoryInsights))
    );
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Git(GitAction::RepositoryInsightsDown))
    );
}

#[test]
fn test_diff_shortcuts_open_external_tool_and_navigate_hunks() {
    let mut state = create_test_state();
    state.focus = FocusPanel::BottomRight;

    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Git(GitAction::OpenExternalDiff))
    );
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Navigation(NavigationAction::NextDiffHunk))
    );
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT),
            &state
        ),
        Some(AppAction::Navigation(NavigationAction::PreviousDiffHunk))
    );

    state.view_mode = ViewMode::Staging;
    state.staging_state.focus = StagingFocus::Diff;
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Git(GitAction::OpenExternalDiff))
    );
}

#[test]
fn test_configured_shortcut_and_custom_command_take_precedence() {
    let mut state = create_test_state();
    let config = crate::config::AppConfig {
        keybindings: std::collections::BTreeMap::from([(
            "graph.inspect".to_string(),
            "ctrl+i".to_string(),
        )]),
        custom_commands: vec![crate::config::CustomCommandConfig {
            name: "Tests".to_string(),
            key: "alt+t".to_string(),
            command: "cargo test".to_string(),
            confirm: true,
            pause: false,
        }],
        ..crate::config::AppConfig::default()
    };
    state.apply_config(&config);

    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL),
            &state,
        ),
        Some(AppAction::Git(GitAction::RepositoryInsights))
    );
    assert_eq!(
        map_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::ALT), &state,),
        Some(AppAction::RunCustomCommand(0))
    );
}

#[test]
fn test_github_pr_shortcut_opens_and_closes_overlay() {
    let mut state = create_test_state();
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT),
            &state
        ),
        Some(AppAction::Git(GitAction::GithubPrStatus))
    );

    state.ui.github_pull_request = Some(crate::git::github::GithubPullRequest {
        number: 1,
        title: "PR".to_string(),
        state: "OPEN".to_string(),
        is_draft: false,
        review_decision: None,
        merge_state_status: None,
        url: "https://example.com/pr/1".to_string(),
        additions: 1,
        deletions: 0,
        changed_files: 1,
        checks: crate::git::github::CheckSummary::default(),
    });
    assert_eq!(
        map_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &state),
        Some(AppAction::Git(GitAction::CloseGithubPrStatus))
    );
}

#[test]
fn test_project_tree_escape_closes_active_branch_comparison() {
    let mut state = create_test_state();
    state.view_mode = ViewMode::ProjectTree;
    state
        .project_tree_state
        .start_comparison("main".to_string(), "feature".to_string());

    let action = map_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &state);

    assert_eq!(action, Some(AppAction::Git(GitAction::ClearComparison)));
}

#[test]
fn test_ctrl_p_triggers_force_push() {
    let state = create_test_state();

    let action = map_key(
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
        &state,
    );

    assert_eq!(action, Some(AppAction::Git(GitAction::ForcePush)));
}

#[test]
fn test_b_switches_to_branches_view_from_graph() {
    let state = create_test_state();

    let action = map_key(
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::SwitchView(ViewMode::Branches)));
}

#[test]
fn test_bottom_left_space_opens_diff_panel() {
    let mut state = create_test_state();
    state.focus = FocusPanel::BottomLeft;

    let action = map_key(
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::Select));
}

#[test]
fn test_bottom_left_enter_opens_fullscreen_diff() {
    let mut state = create_test_state();
    state.focus = FocusPanel::BottomLeft;

    let action = map_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &state);

    assert_eq!(action, Some(AppAction::ToggleDiffFullscreen));
}

#[test]
fn test_staging_space_opens_diff_panel() {
    let mut state = create_test_state();
    state.view_mode = ViewMode::Staging;
    state.staging_state.focus = StagingFocus::Staged;

    let action = map_key(
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::Staging(StagingAction::FocusDiff)));
}

#[test]
fn test_staging_diff_shortcuts_follow_source_list() {
    let mut state = create_test_state();
    state.view_mode = ViewMode::Staging;
    state.staging_state.focus = StagingFocus::Diff;
    state.staging_state.last_file_focus = StagingFocus::Unstaged;

    let stage_hunk = map_key(
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
        &state,
    );
    let stage_line = map_key(
        KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT),
        &state,
    );
    assert_eq!(
        stage_hunk,
        Some(AppAction::Staging(StagingAction::StageHunk))
    );
    assert_eq!(
        stage_line,
        Some(AppAction::Staging(StagingAction::StageLine))
    );

    state.staging_state.last_file_focus = StagingFocus::Staged;
    let unstage_hunk = map_key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE),
        &state,
    );
    let unstage_line = map_key(
        KeyEvent::new(KeyCode::Char('U'), KeyModifiers::SHIFT),
        &state,
    );
    assert_eq!(
        unstage_hunk,
        Some(AppAction::Staging(StagingAction::UnstageHunk))
    );
    assert_eq!(
        unstage_line,
        Some(AppAction::Staging(StagingAction::UnstageLine))
    );
}

#[test]
fn test_mouse_click_nav_bar_switches_view() {
    let state = create_test_state();

    let action = map_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(event::MouseButton::Left),
            column: 20,
            row: 1,
            modifiers: KeyModifiers::NONE,
        },
        &state,
    );

    assert_eq!(action, Some(AppAction::SwitchView(ViewMode::Staging)));
}

#[test]
fn test_four_switches_to_project_tree_view() {
    let state = create_test_state();

    let action = map_key(
        KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::SwitchView(ViewMode::ProjectTree)));
}

#[test]
fn test_project_tree_enter_toggles_selected_directory() {
    let mut state = create_test_state();
    state.view_mode = ViewMode::ProjectTree;

    let action = map_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &state);

    assert_eq!(
        action,
        Some(AppAction::ProjectTree(
            crate::state::ProjectTreeAction::ToggleSelected
        ))
    );
}

#[test]
fn test_five_switches_to_conflicts_when_available() {
    let mut state = create_test_state();
    state.conflicts_state = Some(crate::state::ConflictsState::new(
        Vec::new(),
        "merge".to_string(),
        "main".to_string(),
        "feature".to_string(),
    ));

    let action = map_key(
        KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::SwitchView(ViewMode::Conflicts)));
}

#[test]
fn test_mouse_scroll_graph_moves_down() {
    let state = create_test_state();

    let action = map_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        },
        &state,
    );

    assert_eq!(
        action,
        Some(AppAction::Navigation(NavigationAction::MoveDown))
    );
}

#[test]
fn test_reset_picker_m_key_selects_mixed() {
    let mut state = create_test_state();
    state.reset_picker = Some(ResetPickerState::new(
        git2::Oid::zero(),
        "abc1234".to_string(),
        "Test".to_string(),
    ));

    let action = map_key(
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::ResetPickerSelectMixed));
}
