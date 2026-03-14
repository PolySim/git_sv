use super::keyboard::map_key;
use super::mouse::map_mouse;
use crate::git::repo::GitRepo;
use crate::git::tests::test_utils::create_test_repo;
use crate::state::action::{
    FilterAction, GitAction, NavigationAction, SearchAction, StagingAction,
};
use crate::state::{AppAction, AppState, FocusPanel, StagingFocus, ViewMode};
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
