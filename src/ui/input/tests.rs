use super::keyboard::{map_key, map_paste};
use super::mouse::map_mouse;
use crate::git::repo::GitRepo;
use crate::git::tests::test_utils::create_test_repo;
use crate::state::action::{
    FilterAction, GitAction, HelpAction, NavigationAction, SearchAction, StagingAction,
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
fn comparison_graph_shortcuts_yield_to_configuration_but_comparison_stays_local() {
    use crate::state::action::ComparisonAction;

    let mut state = create_test_state();
    state.customization = crate::config::AppConfig {
        keybindings: [
            ("git.push".into(), "a".into()),
            ("git.pull".into(), "ctrl+a".into()),
            ("git.fetch".into(), "j".into()),
        ]
        .into(),
        custom_commands: vec![crate::config::CustomCommandConfig {
            name: "Local".into(),
            key: "shift+d".into(),
            command: "exit 1".into(),
            confirm: false,
            pause: false,
        }],
        ..Default::default()
    }
    .runtime_customization();
    let cases = [
        (
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            AppAction::Git(GitAction::Push),
        ),
        (
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            AppAction::Git(GitAction::Pull),
        ),
        (
            KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT),
            AppAction::RunCustomCommand(0),
        ),
    ];
    for (key, action) in &cases {
        assert_eq!(map_key(*key, &state).as_ref(), Some(action));
    }
    state.view_mode = ViewMode::Comparison;
    for (key, _) in cases {
        assert_eq!(map_key(key, &state), None);
    }
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            &state
        ),
        Some(AppAction::Comparison(ComparisonAction::LineDown))
    );
}

#[test]
fn comparison_input_is_local_and_does_not_shadow_head_or_text_inputs() {
    use crate::state::action::ComparisonAction;

    let mut state = create_test_state();
    let key = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
    assert_eq!(
        map_key(key('C'), &state),
        Some(AppAction::Git(GitAction::CompareSelectedWithHead))
    );
    assert_eq!(
        map_key(key('D'), &state),
        Some(AppAction::Comparison(ComparisonAction::Open))
    );
    state.search_state.is_active = true;
    assert_eq!(
        map_key(key('a'), &state),
        Some(AppAction::Search(SearchAction::InsertChar('a')))
    );
    state.search_state.is_active = false;
    state.customization = crate::config::AppConfig {
        keybindings: [("git.push".into(), "x".into())].into(),
        custom_commands: vec![crate::config::CustomCommandConfig {
            name: "Local".into(),
            key: "z".into(),
            command: "exit 1".into(),
            confirm: false,
            pause: false,
        }],
        ..Default::default()
    }
    .runtime_customization();
    assert_eq!(
        map_key(key('x'), &state),
        Some(AppAction::Git(GitAction::Push))
    );
    assert_eq!(
        map_key(key('z'), &state),
        Some(AppAction::RunCustomCommand(0))
    );
    state.view_mode = ViewMode::Comparison;
    for c in ['x', 'z', 'p', 'P', 'f', '1', '2', 'C', 'D', 'r', '?'] {
        assert_eq!(map_key(key(c), &state), None, "touche {c}");
    }
    assert_eq!(map_paste("p".into(), &state), None);
    for kind in [
        MouseEventKind::Down(event::MouseButton::Left),
        MouseEventKind::ScrollDown,
    ] {
        assert_eq!(
            map_mouse(
                MouseEvent {
                    kind,
                    column: 15,
                    row: 1,
                    modifiers: KeyModifiers::NONE
                },
                &state
            ),
            None
        );
    }
    let mut dispatcher = crate::handler::dispatcher::ActionDispatcher::new();
    dispatcher
        .dispatch(&mut state, AppAction::SwitchView(ViewMode::Staging))
        .unwrap();
    dispatcher
        .dispatch(&mut state, AppAction::RunCustomCommand(0))
        .unwrap();
    assert_eq!(state.view_mode, ViewMode::Comparison);
    assert!(state.ui.pending_custom_command.is_none());
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
fn help_input_is_local_and_restores_the_previous_view() {
    use crate::handler::dispatcher::ActionDispatcher;

    let mut state = create_test_state();
    let mut dispatcher = ActionDispatcher::new();
    state.view_mode = ViewMode::Staging;
    state.dirty = false;
    state.customization = crate::config::AppConfig {
        keybindings: [
            ("git.push".into(), "x".into()),
            ("global.help".into(), "alt+h".into()),
        ]
        .into(),
        custom_commands: vec![crate::config::CustomCommandConfig {
            name: "Local".into(),
            key: "z".into(),
            command: "exit 1".into(),
            confirm: false,
            pause: false,
        }],
        ..Default::default()
    }
    .runtime_customization();
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    let dispatch_key = |dispatcher: &mut ActionDispatcher, state: &mut AppState, code| {
        let action = map_key(key(code), state).unwrap();
        dispatcher.dispatch(state, action).unwrap();
    };
    let open = map_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::ALT), &state).unwrap();
    assert_eq!(open, AppAction::ToggleHelp);
    dispatcher.dispatch(&mut state, open).unwrap();
    for c in ['x', 'y', 'z'] {
        assert_eq!(map_key(key(KeyCode::Char(c)), &state), None);
    }
    assert_eq!(
        map_key(key(KeyCode::PageDown), &state),
        Some(AppAction::Help(HelpAction::PageDown))
    );
    assert_eq!(map_paste("x".into(), &state), None);
    dispatch_key(&mut dispatcher, &mut state, KeyCode::Char('/'));
    for c in ['x', 'y', 'z', '?', 'j', '/'] {
        assert_eq!(
            map_key(key(KeyCode::Char(c)), &state),
            Some(AppAction::Help(HelpAction::Insert(c.to_string())))
        );
        dispatch_key(&mut dispatcher, &mut state, KeyCode::Char(c));
    }
    dispatch_key(&mut dispatcher, &mut state, KeyCode::Backspace);
    let paste = map_paste("\nP\tq\u{1b}".into(), &state).unwrap();
    dispatcher.dispatch(&mut state, paste).unwrap();
    assert_eq!(state.help.query, "xyz?j P q");
    dispatch_key(&mut dispatcher, &mut state, KeyCode::Enter);
    assert!(!state.help.searching);
    assert_eq!(state.view_mode, ViewMode::Help);
    assert!(!state.dirty);
    dispatch_key(&mut dispatcher, &mut state, KeyCode::Char('/'));
    dispatch_key(&mut dispatcher, &mut state, KeyCode::Esc);
    assert_eq!(state.help.query, "xyz?j P q");
    assert_eq!(state.view_mode, ViewMode::Help);
    dispatch_key(&mut dispatcher, &mut state, KeyCode::Esc);
    assert_eq!(state.view_mode, ViewMode::Staging);
    dispatch_key(&mut dispatcher, &mut state, KeyCode::Char('?'));
    assert!(state.help.query.is_empty());
}

#[test]
fn help_wheel_is_scoped_to_the_popup_and_yields_to_a_modal() {
    let mut state = create_test_state();
    state.toggle_help();
    state.help.searching = true;
    let popup = crate::ui::help_overlay::popup_area(state.screen_area);
    let mouse = |kind, column, row| MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    let wheel = mouse(MouseEventKind::ScrollDown, popup.x + 1, popup.y + 2);
    assert_eq!(
        map_mouse(wheel, &state),
        Some(AppAction::Help(HelpAction::Down))
    );
    assert_eq!(
        map_mouse(mouse(MouseEventKind::ScrollDown, 0, 0), &state),
        None
    );
    assert_eq!(
        map_mouse(
            mouse(MouseEventKind::Down(event::MouseButton::Left), 0, 0),
            &state
        ),
        None
    );
    state.open_confirmation(crate::ui::confirm_dialog::ConfirmAction::DiscardAll);
    assert_eq!(map_mouse(wheel, &state), None);
    assert_eq!(map_paste("y".into(), &state), None);
    assert_eq!(
        map_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &state),
        Some(AppAction::CancelAction)
    );
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
fn branch_finder_routes_text_and_mouse_without_leaking_shortcuts() {
    use crate::state::action::{BranchAction, BranchFinderAction};
    use crate::state::{BranchesFocus, BranchesSection};
    use crate::test_utils::ui_driver::UiTestHarness;
    use crossterm::event::KeyEventKind;

    let mut harness = UiTestHarness::new();
    harness.state.view_mode = ViewMode::Branches;
    harness.state.branches_view_state.section = BranchesSection::Worktrees;
    assert_eq!(
        map_key(
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
            &harness.state
        ),
        None
    );
    harness.state.branches_view_state.section = BranchesSection::Branches;
    harness.state.branches_view_state.focus = BranchesFocus::Detail;
    harness.state.customization = crate::config::AppConfig {
        keybindings: [("git.push".into(), "z".into())].into(),
        ..Default::default()
    }
    .runtime_customization();
    harness.send_char('/');
    harness.send_text("qdR3?z/");
    assert_eq!(
        harness.state.branches_view_state.finder().unwrap().query,
        "qdR3?z/"
    );
    let key = |code, modifiers, kind| KeyEvent::new_with_kind(code, modifiers, kind);
    let map = |event| map_key(event, &harness.state);
    assert_eq!(
        map(key(KeyCode::Tab, KeyModifiers::NONE, KeyEventKind::Press)),
        None
    );
    assert_eq!(
        map(key(
            KeyCode::Right,
            KeyModifiers::NONE,
            KeyEventKind::Repeat
        )),
        None
    );
    assert_eq!(
        map(key(KeyCode::Esc, KeyModifiers::NONE, KeyEventKind::Release)),
        None
    );
    assert_eq!(
        map(key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press
        )),
        Some(AppAction::Quit)
    );
    assert_eq!(
        map(key(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Repeat)),
        Some(AppAction::Branch(BranchAction::Finder(
            BranchFinderAction::Down
        )))
    );

    let popup = super::super::branch_finder::popup_area(harness.state.screen_area);
    let mouse = |kind, column, row| MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    let click = MouseEventKind::Down(event::MouseButton::Left);
    assert_eq!(
        map_mouse(mouse(click, popup.x + 1, popup.y + 3), &harness.state),
        None
    );
    assert_eq!(map_mouse(mouse(click, 2, 7), &harness.state), None);
    assert_eq!(
        map_mouse(mouse(MouseEventKind::ScrollDown, 2, 7), &harness.state),
        None
    );
    assert_eq!(
        map_mouse(
            mouse(MouseEventKind::ScrollDown, popup.x + 1, popup.y + 3),
            &harness.state
        ),
        Some(AppAction::Branch(BranchAction::Finder(
            BranchFinderAction::Down
        )))
    );
    harness.send_esc();
    assert_eq!(
        harness.state.branches_view_state.focus,
        BranchesFocus::Detail
    );
    assert!(harness.state.branches_view_state.finder().is_none());
    assert_eq!(map_paste("main".into(), &harness.state), None);
    harness.send_char('/');
    assert!(harness
        .state
        .branches_view_state
        .finder()
        .unwrap()
        .query
        .is_empty());
}

#[test]
fn branch_finder_selects_without_checkout_and_refresh_keeps_action_target() {
    use crate::state::BranchesFocus;
    use crate::test_utils::ui_driver::UiTestHarness;

    let mut harness = UiTestHarness::new();
    let add_branch = |state: &AppState, name| {
        let repo = &state.repo.repo;
        let tip = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch(name, &tip, false).unwrap();
    };
    add_branch(&harness.state, "topic/release");
    harness.state.enter_view(ViewMode::Branches);
    harness.state.refresh_from_repo().unwrap();
    harness.state.branches_view_state.focus = BranchesFocus::Detail;
    harness.send_char('/');
    let paste = map_paste("\nTOPIC/release\r\n".into(), &harness.state).unwrap();
    harness.dispatch(paste);
    assert_eq!(
        harness
            .state
            .branches_view_state
            .finder()
            .unwrap()
            .matches
            .len(),
        1
    );
    assert_eq!(
        harness
            .state
            .branches_view_state
            .selected_branch()
            .unwrap()
            .name,
        "main"
    );
    assert!(!harness.state.dirty);
    // Certains terminaux ne distinguent pas les répétitions des Press.
    for _ in 0..3 {
        harness.send_enter();
    }
    assert!(harness.state.branches_view_state.finder().is_some());
    for _ in 0..3 {
        harness.send_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    }
    assert!(!harness.state.dirty);
    assert_eq!(
        harness.state.repo.repo.head().unwrap().shorthand(),
        Ok("main")
    );
    assert!(harness.state.branches_view_state.finder().is_none());
    assert_eq!(harness.state.branches_view_state.focus, BranchesFocus::List);
    assert_eq!(
        harness
            .state
            .branches_view_state
            .selected_branch()
            .unwrap()
            .name,
        "topic/release"
    );
    add_branch(&harness.state, "a-new-branch");
    harness.state.refresh_from_repo().unwrap();
    assert_eq!(harness.state.branches_view_state.local_branches.len(), 3);
    harness.send_char('r');
    assert_eq!(
        harness.state.branches_view_state.focus,
        BranchesFocus::Input
    );
    assert_eq!(
        harness.state.branches_view_state.input_text,
        "topic/release"
    );
    harness.send_esc();
    harness.send_enter();
    assert_eq!(
        harness.state.repo.repo.head().unwrap().shorthand(),
        Ok("topic/release")
    );
}

#[test]
fn branch_finder_accept_key_cannot_leak_to_a_configured_checkout() {
    use crate::test_utils::ui_driver::UiTestHarness;

    let mut harness = UiTestHarness::new();
    harness.state.enter_view(ViewMode::Branches);
    harness.state.refresh_from_repo().unwrap();
    harness.state.customization = crate::config::AppConfig {
        keybindings: [("branches.checkout".into(), "right".into())].into(),
        ..Default::default()
    }
    .runtime_customization();
    let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    harness.send_char('/');
    harness.send_key(right);
    assert!(harness.state.branches_view_state.finder().is_none());
    assert_eq!(map_key(right, &harness.state), None);
}

#[test]
fn branch_finder_restores_click_targets_in_the_scrolled_branch_list() {
    use crate::git::branch::BranchInfo;
    use crate::state::action::BranchAction;
    use crate::state::SelectedBranch;
    use crate::test_utils::ui_driver::UiTestHarness;
    use ratatui::{backend::TestBackend, Terminal};

    let mut harness = UiTestHarness::new();
    harness.state.view_mode = ViewMode::Branches;
    let branches = &mut harness.state.branches_view_state;
    branches.toggle_remote();
    branches.replace_branches(
        (0..24)
            .map(|i| BranchInfo::simple(format!("topic/{i:02}"), false, false))
            .collect(),
        vec![BranchInfo::simple("origin/topic/24".into(), false, true)],
    );
    harness.send_char('/');
    harness.send_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    harness.send_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(
        harness.state.branches_view_state.selected_branch,
        Some(SelectedBranch::Remote(0))
    );

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(frame, &mut harness.state))
        .unwrap();
    let panel =
        crate::ui::branches_layout::build_branches_layout(harness.state.screen_area).list_panel;
    let buffer = terminal.backend().buffer();
    let rendered_row = |y| {
        (panel.x..panel.right())
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    };
    let local = (panel.y..panel.bottom())
        .find(|&y| rendered_row(y).contains("topic/23"))
        .unwrap();
    let remote = (panel.y..panel.bottom())
        .find(|&y| rendered_row(y).contains("origin/topic/24"))
        .unwrap();
    let click = |column, row| {
        map_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(event::MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            },
            &harness.state,
        )
    };
    assert_eq!(
        click(panel.x + 3, remote),
        Some(AppAction::Branch(BranchAction::SelectRemoteBranch(0)))
    );
    assert_eq!(click(panel.x, local), None);
    let target = click(panel.x + 3, local).unwrap();
    harness.dispatch(target);
    assert_eq!(
        harness.state.branches_view_state.selected_branch,
        Some(SelectedBranch::Local(23))
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
        commit: git2::Oid::ZERO_SHA1.to_string(),
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
fn test_mouse_selects_the_rendered_staging_file_after_scrolling() {
    let mut state = create_test_state();
    state.view_mode = ViewMode::Staging;
    state.staging_state.set_unstaged_files(
        (0..40)
            .map(|index| crate::git::repo::StatusEntry {
                path: format!("file-{index:04}.txt"),
                status: git2::Status::WT_NEW,
            })
            .collect(),
    );
    state.staging_state.set_unstaged_selected(30);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(frame, &mut state))
        .unwrap();
    let panel = crate::ui::staging_layout::build_staging_layout(state.screen_area).unstaged_panel;
    let displayed: String = (panel.x..panel.right())
        .map(|x| terminal.backend().buffer()[(x, panel.y + 1)].symbol())
        .collect();
    let action = map_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(event::MouseButton::Left),
            column: panel.x + 2,
            row: panel.y + 1,
            modifiers: KeyModifiers::NONE,
        },
        &state,
    );
    let Some(AppAction::Staging(StagingAction::SelectUnstaged(index))) = action else {
        panic!("Le clic doit sélectionner le fichier affiché");
    };
    assert!(index > 0);
    assert!(
        displayed.contains(&state.staging_state.unstaged_files()[index].path),
        "{displayed}"
    );
}

#[test]
fn test_mouse_scroll_targets_hovered_staging_list() {
    let mut state = create_test_state();
    state.view_mode = ViewMode::Staging;
    state.staging_state.focus = StagingFocus::Diff;
    state.staging_state.set_staged_files(
        (0..3)
            .map(|index| crate::git::repo::StatusEntry {
                path: format!("file-{index}.txt"),
                status: git2::Status::INDEX_NEW,
            })
            .collect(),
    );
    let panel = crate::ui::staging_layout::build_staging_layout(state.screen_area).staged_panel;
    let action = map_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: panel.x + 2,
            row: panel.y + 1,
            modifiers: KeyModifiers::NONE,
        },
        &state,
    );
    assert_eq!(
        action,
        Some(AppAction::Staging(StagingAction::SelectStaged(1)))
    );
}

#[test]
fn test_mouse_cannot_navigate_behind_modal_overlays() {
    let mut state = create_test_state();
    state.ui.repository_insights = Some(crate::git::insights::RepositoryInsights {
        commit: String::new(),
        signature: crate::git::insights::CommitSignatureStatus::Unsigned,
        hooks: Vec::new(),
        submodules: Vec::new(),
    });
    for kind in [
        MouseEventKind::Down(event::MouseButton::Left),
        MouseEventKind::ScrollDown,
    ] {
        assert_eq!(
            map_mouse(
                MouseEvent {
                    kind,
                    column: 20,
                    row: 1,
                    modifiers: KeyModifiers::NONE,
                },
                &state
            ),
            None
        );
    }
    state.ui.repository_insights = None;
    state
        .ui
        .open_confirmation(crate::ui::confirm_dialog::ConfirmAction::DiscardAll);
    assert_eq!(
        map_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 20,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &state
        ),
        None
    );
}

#[test]
fn test_confirmation_mouse_only_accepts_the_visible_yes_button() {
    for width in [100, 101, 120, 121] {
        let mut state = create_test_state();
        state.view_mode = ViewMode::Staging;
        state.open_confirmation(crate::ui::confirm_dialog::ConfirmAction::DiscardAll);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 40)).unwrap();
        terminal
            .draw(|frame| crate::ui::render(frame, &mut state))
            .unwrap();
        let popup = crate::ui::common::centered_rect(60, 30, state.screen_area);
        let row = popup.y + 4;
        let yes_x = (popup.x..popup.right())
            .find(|&x| terminal.backend().buffer()[(x, row)].symbol() == "y")
            .unwrap();
        let mouse = |column, row| MouseEvent {
            kind: MouseEventKind::Down(event::MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            map_mouse(mouse(yes_x, row), &state),
            Some(AppAction::ConfirmAction)
        );
        assert_ne!(
            map_mouse(mouse(popup.x + 2, popup.bottom() - 2), &state),
            Some(AppAction::ConfirmAction)
        );
    }
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
        git2::Oid::ZERO_SHA1,
        "abc1234".to_string(),
        "Test".to_string(),
    ));

    let action = map_key(
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
        &state,
    );

    assert_eq!(action, Some(AppAction::ResetPickerSelectMixed));
}
