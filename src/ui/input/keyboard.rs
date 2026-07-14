use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::state::action::{
    BranchAction, ConflictAction, EditAction, FilterAction, GitAction, NavigationAction,
    SearchAction, StagingAction,
};
use crate::state::{
    AppAction, AppState, BranchesFocus, BranchesSection, ConflictPanelFocus, FocusPanel,
    ProjectTreeAction, ProjectTreeFocus, StagingFocus, ViewMode,
};

pub(crate) fn map_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    if let Some(action) = map_quit_shortcut(key) {
        return Some(action);
    }

    if has_blocking_modal(state) {
        return map_modal_key(key, state);
    }

    if has_blocking_text_input(state) {
        return map_text_input_key(key, state);
    }

    if state.view_mode == ViewMode::Help {
        return map_help_key(key, state);
    }

    map_view_switch_key(key, state).or_else(|| map_view_key(key, state))
}

#[cfg(test)]
pub(crate) fn map_key_for_test(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    map_key(key, state)
}

fn map_quit_shortcut(key: KeyEvent) -> Option<AppAction> {
    (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        .then_some(AppAction::Quit)
}

fn has_blocking_modal(state: &AppState) -> bool {
    state
        .merge_picker
        .as_ref()
        .is_some_and(|picker| picker.is_active)
        || state
            .reset_picker
            .as_ref()
            .is_some_and(|picker| picker.is_active)
        || state.ui.repository_insights.is_some()
        || state.ui.pending_confirmation.is_some()
}

fn has_blocking_text_input(state: &AppState) -> bool {
    state.search_state.is_active
        || (state.view_mode == ViewMode::ProjectTree && state.project_tree_state.search.is_active)
        || state.filters.filter_popup.is_open
        || (state.view_mode == ViewMode::Staging
            && state.staging_state.focus == StagingFocus::CommitMessage)
        || state.branches_view_state.focus == BranchesFocus::Input
        || (state.view_mode == ViewMode::Conflicts
            && state
                .conflicts_state
                .as_ref()
                .is_some_and(|conflicts| conflicts.is_editing))
}

fn map_modal_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    if state.ui.repository_insights.is_some() {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('i') => {
                Some(AppAction::Git(GitAction::RepositoryInsights))
            }
            KeyCode::Char('j') | KeyCode::Down | KeyCode::PageDown => {
                Some(AppAction::Git(GitAction::RepositoryInsightsDown))
            }
            KeyCode::Char('k') | KeyCode::Up | KeyCode::PageUp => {
                Some(AppAction::Git(GitAction::RepositoryInsightsUp))
            }
            _ => None,
        };
    }

    if state
        .merge_picker
        .as_ref()
        .is_some_and(|picker| picker.is_active)
    {
        return map_merge_picker_key(key);
    }

    if state
        .reset_picker
        .as_ref()
        .is_some_and(|picker| picker.is_active)
    {
        return map_reset_picker_key(key);
    }

    if state.ui.pending_confirmation.is_some() {
        return map_confirmation_key(key);
    }

    None
}

fn map_text_input_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    if state.view_mode == ViewMode::ProjectTree && state.project_tree_state.search.is_active {
        return map_project_tree_search_key(key);
    }

    if state.search_state.is_active {
        return map_search_key(key);
    }

    if state.filters.filter_popup.is_open {
        return map_filter_popup_key(key);
    }

    if state.view_mode == ViewMode::Staging
        && state.staging_state.focus == StagingFocus::CommitMessage
    {
        return map_staging_commit_input_key(key);
    }

    if state.branches_view_state.focus == BranchesFocus::Input {
        return map_branches_input_key(key);
    }

    if state.view_mode == ViewMode::Conflicts {
        return map_conflicts_editing_key(key, state);
    }

    None
}

fn map_help_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    if state.view_mode != ViewMode::Help {
        return None;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('?') => Some(AppAction::ToggleHelp),
        _ => None,
    }
}

fn map_view_switch_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('1') => Some(AppAction::SwitchView(ViewMode::Graph)),
        KeyCode::Char('2') => Some(AppAction::SwitchView(ViewMode::Staging)),
        KeyCode::Char('3') => Some(AppAction::SwitchView(ViewMode::Branches)),
        KeyCode::Char('4') => Some(AppAction::SwitchView(ViewMode::ProjectTree)),
        KeyCode::Char('5') if state.conflicts_state.is_some() => {
            Some(AppAction::SwitchView(ViewMode::Conflicts))
        }
        KeyCode::Char('w') => Some(AppAction::Branch(BranchAction::OpenWorktrees)),
        _ => None,
    }
}

fn map_view_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match state.view_mode {
        ViewMode::Graph => map_graph_key(key, state),
        ViewMode::Staging => map_staging_key(key, state),
        ViewMode::Branches => map_branches_key(key, state),
        ViewMode::ProjectTree => map_project_tree_key(key, state),
        ViewMode::Conflicts => map_conflicts_key(key, state),
        ViewMode::Blame => map_blame_key(key),
        ViewMode::Help => None,
    }
}

fn map_project_tree_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    if state.project_tree_state.focus == ProjectTreeFocus::Diff {
        if key.code == KeyCode::Char('e') {
            return Some(AppAction::Git(GitAction::OpenExternalDiff));
        }
        let diff_action = match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(NavigationAction::ScrollDiffDown),
            KeyCode::Char('k') | KeyCode::Up => Some(NavigationAction::ScrollDiffUp),
            KeyCode::Char('h') | KeyCode::Left => Some(NavigationAction::ScrollDiffLeft),
            KeyCode::Char('l') | KeyCode::Right => Some(NavigationAction::ScrollDiffRight),
            KeyCode::Char('g') | KeyCode::Home => Some(NavigationAction::ScrollDiffTop),
            KeyCode::Char('G') | KeyCode::End => Some(NavigationAction::ScrollDiffBottom),
            KeyCode::PageUp => Some(NavigationAction::ScrollDiffPageUp),
            KeyCode::PageDown => Some(NavigationAction::ScrollDiffPageDown),
            KeyCode::Char('n') => Some(NavigationAction::NextDiffHunk),
            KeyCode::Char('N') => Some(NavigationAction::PreviousDiffHunk),
            _ => None,
        };
        if let Some(action) = diff_action {
            return Some(AppAction::Navigation(action));
        }
    }

    if key.code == KeyCode::Esc && state.project_tree_state.comparison.is_some() {
        return Some(AppAction::Git(GitAction::ClearComparison));
    }

    match key.code {
        KeyCode::Char('q') => Some(AppAction::Quit),
        KeyCode::Char('j') | KeyCode::Down => {
            Some(AppAction::Navigation(NavigationAction::MoveDown))
        }
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::Navigation(NavigationAction::MoveUp)),
        KeyCode::Char('g') | KeyCode::Home => Some(AppAction::Navigation(NavigationAction::GoTop)),
        KeyCode::Char('G') | KeyCode::End => {
            Some(AppAction::Navigation(NavigationAction::GoBottom))
        }
        KeyCode::PageUp => Some(AppAction::Navigation(NavigationAction::PageUp)),
        KeyCode::PageDown => Some(AppAction::Navigation(NavigationAction::PageDown)),
        KeyCode::Enter | KeyCode::Char(' ')
            if state.project_tree_state.focus == ProjectTreeFocus::Tree =>
        {
            Some(AppAction::ProjectTree(ProjectTreeAction::ToggleSelected))
        }
        KeyCode::Left | KeyCode::Char('h')
            if state.project_tree_state.focus == ProjectTreeFocus::Tree =>
        {
            Some(AppAction::ProjectTree(ProjectTreeAction::CollapseSelected))
        }
        KeyCode::Right | KeyCode::Char('l')
            if state.project_tree_state.focus == ProjectTreeFocus::Tree =>
        {
            Some(AppAction::ProjectTree(ProjectTreeAction::ExpandSelected))
        }
        KeyCode::Char('/') => Some(AppAction::ProjectTree(ProjectTreeAction::OpenSearch)),
        KeyCode::Char('C') => Some(AppAction::Git(GitAction::ComparePrompt)),
        KeyCode::Char('e') if state.project_tree_state.focus == ProjectTreeFocus::ChangedFiles => {
            Some(AppAction::Git(GitAction::OpenExternalDiff))
        }
        KeyCode::Char('v') if state.project_tree_state.focus == ProjectTreeFocus::Diff => {
            Some(AppAction::ToggleDiffViewMode)
        }
        KeyCode::Tab | KeyCode::BackTab => {
            Some(AppAction::Navigation(NavigationAction::SwitchPanel))
        }
        KeyCode::Char('r') => Some(AppAction::Refresh),
        KeyCode::Char('y') => Some(AppAction::CopyPanelContent),
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),
        _ => None,
    }
}

fn map_project_tree_search_key(key: KeyEvent) -> Option<AppAction> {
    let action = match key.code {
        KeyCode::Esc => Some(ProjectTreeAction::CloseSearch),
        KeyCode::Enter => Some(ProjectTreeAction::ConfirmSearch),
        KeyCode::Down if key.modifiers.is_empty() => Some(ProjectTreeAction::SearchNext),
        KeyCode::Up if key.modifiers.is_empty() => Some(ProjectTreeAction::SearchPrevious),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(ProjectTreeAction::SearchNext)
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(ProjectTreeAction::SearchPrevious)
        }
        _ => None,
    };

    action.map(AppAction::ProjectTree).or_else(|| {
        map_text_edit_key(key)
            .map(ProjectTreeAction::EditSearch)
            .map(AppAction::ProjectTree)
    })
}

fn map_merge_picker_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MergePickerDown),
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MergePickerUp),
        KeyCode::Enter => Some(AppAction::MergePickerConfirm),
        KeyCode::Esc => Some(AppAction::MergePickerCancel),
        _ => None,
    }
}

fn map_reset_picker_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(AppAction::ResetPickerDown),
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::ResetPickerUp),
        KeyCode::Char('s') => Some(AppAction::ResetPickerSelectSoft),
        KeyCode::Char('m') => Some(AppAction::ResetPickerSelectMixed),
        KeyCode::Char('h') => Some(AppAction::ResetPickerSelectHard),
        KeyCode::Enter => Some(AppAction::ResetPickerConfirm),
        KeyCode::Esc => Some(AppAction::ResetPickerCancel),
        _ => None,
    }
}

fn map_confirmation_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('y' | 'Y') => Some(AppAction::ConfirmAction),
        KeyCode::Char('n' | 'N') | KeyCode::Esc => Some(AppAction::CancelAction),
        _ => None,
    }
}

fn map_search_key(key: KeyEvent) -> Option<AppAction> {
    let action = match key.code {
        KeyCode::Esc => Some(AppAction::Search(SearchAction::Close)),
        KeyCode::Enter => Some(AppAction::Search(SearchAction::Execute)),
        KeyCode::Down if key.modifiers.is_empty() => {
            Some(AppAction::Search(SearchAction::NextResult))
        }
        KeyCode::Up if key.modifiers.is_empty() => {
            Some(AppAction::Search(SearchAction::PreviousResult))
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Search(SearchAction::NextResult))
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Search(SearchAction::PreviousResult))
        }
        KeyCode::Tab => Some(AppAction::Search(SearchAction::ChangeType)),
        _ => None,
    };

    action.or_else(|| {
        map_text_edit_key(key).map(|action| match action {
            EditAction::InsertChar(c) => AppAction::Search(SearchAction::InsertChar(c)),
            EditAction::DeleteCharBefore => AppAction::Search(SearchAction::DeleteChar),
            action => AppAction::Search(SearchAction::Edit(action)),
        })
    })
}

fn map_text_edit_key(key: KeyEvent) -> Option<EditAction> {
    let command = key
        .modifiers
        .intersects(KeyModifiers::SUPER | KeyModifiers::META);
    let shortcut = command || key.modifiers.contains(KeyModifiers::CONTROL);
    let option = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        KeyCode::Char('z' | 'Z') if shortcut && shift => Some(EditAction::Redo),
        KeyCode::Char('z' | 'Z') if shortcut => Some(EditAction::Undo),
        KeyCode::Char('y' | 'Y') if shortcut => Some(EditAction::Redo),
        KeyCode::Char('a' | 'A') if shortcut => Some(EditAction::SelectAll),
        KeyCode::Left if command && shift => Some(EditAction::SelectHome),
        KeyCode::Right if command && shift => Some(EditAction::SelectEnd),
        KeyCode::Up if command && shift => Some(EditAction::SelectHome),
        KeyCode::Down if command && shift => Some(EditAction::SelectEnd),
        KeyCode::Left if command => Some(EditAction::CursorHome),
        KeyCode::Right if command => Some(EditAction::CursorEnd),
        KeyCode::Up if command => Some(EditAction::CursorHome),
        KeyCode::Down if command => Some(EditAction::CursorEnd),
        KeyCode::Left if option && shift => Some(EditAction::SelectWordLeft),
        KeyCode::Right if option && shift => Some(EditAction::SelectWordRight),
        KeyCode::Up if option && shift => Some(EditAction::SelectHome),
        KeyCode::Down if option && shift => Some(EditAction::SelectEnd),
        KeyCode::Left if option => Some(EditAction::CursorWordLeft),
        KeyCode::Right if option => Some(EditAction::CursorWordRight),
        KeyCode::Up if option => Some(EditAction::CursorHome),
        KeyCode::Down if option => Some(EditAction::CursorEnd),
        KeyCode::Left if shift => Some(EditAction::SelectLeft),
        KeyCode::Right if shift => Some(EditAction::SelectRight),
        KeyCode::Home if shift => Some(EditAction::SelectHome),
        KeyCode::End if shift => Some(EditAction::SelectEnd),
        KeyCode::Left => Some(EditAction::CursorLeft),
        KeyCode::Right => Some(EditAction::CursorRight),
        KeyCode::Home => Some(EditAction::CursorHome),
        KeyCode::End => Some(EditAction::CursorEnd),
        KeyCode::Backspace if command => Some(EditAction::DeleteToStart),
        KeyCode::Backspace if option => Some(EditAction::DeleteWordBefore),
        KeyCode::Backspace => Some(EditAction::DeleteCharBefore),
        KeyCode::Delete if command => Some(EditAction::DeleteToEnd),
        KeyCode::Delete => Some(EditAction::DeleteCharAfter),
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER | KeyModifiers::META) =>
        {
            Some(EditAction::InsertChar(c))
        }
        _ => None,
    }
}

fn map_filter_popup_key(key: KeyEvent) -> Option<AppAction> {
    let action = match key.code {
        KeyCode::Esc => Some(AppAction::Filter(FilterAction::Close)),
        KeyCode::Enter => Some(AppAction::Filter(FilterAction::Apply)),
        KeyCode::Tab | KeyCode::Down if key.modifiers.is_empty() => {
            Some(AppAction::Filter(FilterAction::NextField))
        }
        KeyCode::BackTab => Some(AppAction::Filter(FilterAction::PreviousField)),
        KeyCode::Up if key.modifiers.is_empty() => {
            Some(AppAction::Filter(FilterAction::PreviousField))
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Filter(FilterAction::Clear))
        }
        _ => None,
    };

    action.or_else(|| {
        map_text_edit_key(key).map(|action| match action {
            EditAction::InsertChar(c) => AppAction::Filter(FilterAction::InsertChar(c)),
            EditAction::DeleteCharBefore => AppAction::Filter(FilterAction::DeleteChar),
            action => AppAction::Filter(FilterAction::Edit(action)),
        })
    })
}

fn map_staging_commit_input_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Enter => Some(AppAction::Staging(StagingAction::ConfirmCommit)),
        KeyCode::Esc => Some(AppAction::Staging(StagingAction::CancelCommit)),
        _ => map_text_edit_key(key).map(AppAction::Edit),
    }
}

fn map_branches_input_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Enter => Some(AppAction::Branch(BranchAction::ConfirmInput)),
        KeyCode::Esc => Some(AppAction::Branch(BranchAction::CancelInput)),
        _ => map_text_edit_key(key).map(AppAction::Edit),
    }
}

fn map_graph_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    map_graph_ctrl_key(key, state)
        .or_else(|| map_graph_escape_key(key, state))
        .or_else(|| map_graph_focus_key(key, state))
        .or_else(|| map_graph_root_key(key, state))
}

fn map_graph_ctrl_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }

    match key.code {
        KeyCode::Char('p') => Some(AppAction::Git(GitAction::ForcePush)),
        KeyCode::Char('d') if state.focus == FocusPanel::BottomRight => {
            Some(AppAction::Navigation(NavigationAction::ScrollDiffPageDown))
        }
        KeyCode::Char('d') => Some(AppAction::Navigation(NavigationAction::PageDown)),
        KeyCode::Char('u') if state.focus == FocusPanel::BottomRight => {
            Some(AppAction::Navigation(NavigationAction::ScrollDiffPageUp))
        }
        KeyCode::Char('u') => Some(AppAction::Navigation(NavigationAction::PageUp)),
        KeyCode::Char('r') if state.filters.graph_filter.is_active() => {
            Some(AppAction::Filter(FilterAction::Clear))
        }
        _ => None,
    }
}

fn map_graph_escape_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    if key.code != KeyCode::Esc {
        return None;
    }

    if state.graph_view.diff_fullscreen {
        return Some(AppAction::ToggleDiffFullscreen);
    }

    match state.focus {
        FocusPanel::BottomRight => Some(AppAction::SwitchBottomMode),
        FocusPanel::BottomLeft => Some(AppAction::Navigation(NavigationAction::BackToGraph)),
        FocusPanel::Graph => None,
    }
}

fn map_graph_focus_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match state.focus {
        FocusPanel::BottomLeft => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::FileDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::FileUp))
            }
            KeyCode::Char(' ') => Some(AppAction::Select),
            KeyCode::Char('z') | KeyCode::Enter => Some(AppAction::ToggleDiffFullscreen),
            KeyCode::Char('e') => Some(AppAction::Git(GitAction::OpenExternalDiff)),
            _ => None,
        },
        FocusPanel::BottomRight => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffUp))
            }
            KeyCode::Char('h') | KeyCode::Left => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffLeft))
            }
            KeyCode::Char('l') | KeyCode::Right => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffRight))
            }
            KeyCode::Char('g') | KeyCode::Home => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffTop))
            }
            KeyCode::Char('G') | KeyCode::End => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffBottom))
            }
            KeyCode::Char('z') | KeyCode::Enter => Some(AppAction::ToggleDiffFullscreen),
            KeyCode::Char('v') => Some(AppAction::ToggleDiffViewMode),
            KeyCode::Char('e') => Some(AppAction::Git(GitAction::OpenExternalDiff)),
            KeyCode::Char('n') => Some(AppAction::Navigation(NavigationAction::NextDiffHunk)),
            KeyCode::Char('N') => Some(AppAction::Navigation(NavigationAction::PreviousDiffHunk)),
            KeyCode::PageUp => Some(AppAction::Navigation(NavigationAction::ScrollDiffPageUp)),
            KeyCode::PageDown => Some(AppAction::Navigation(NavigationAction::ScrollDiffPageDown)),
            _ => None,
        },
        FocusPanel::Graph => None,
    }
}

fn map_graph_root_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('q') => Some(AppAction::Quit),
        KeyCode::Char('j') | KeyCode::Down => {
            Some(AppAction::Navigation(NavigationAction::MoveDown))
        }
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::Navigation(NavigationAction::MoveUp)),
        KeyCode::Char('g') | KeyCode::Home => Some(AppAction::Navigation(NavigationAction::GoTop)),
        KeyCode::Char('G') | KeyCode::End => {
            Some(AppAction::Navigation(NavigationAction::GoBottom))
        }
        KeyCode::PageUp => Some(AppAction::Navigation(NavigationAction::PageUp)),
        KeyCode::PageDown => Some(AppAction::Navigation(NavigationAction::PageDown)),
        KeyCode::Enter => Some(AppAction::Select),
        KeyCode::Char('c') => Some(AppAction::Git(GitAction::CommitPrompt)),
        KeyCode::Char('s') => Some(AppAction::Git(GitAction::StashPrompt)),
        KeyCode::Char('m') => Some(AppAction::Git(GitAction::MergePrompt)),
        KeyCode::Char('b') => Some(AppAction::SwitchView(ViewMode::Branches)),
        KeyCode::Char('P') => Some(AppAction::Git(GitAction::Push)),
        KeyCode::Char('p') => Some(AppAction::Git(GitAction::Pull)),
        KeyCode::Char('f') => Some(AppAction::Git(GitAction::Fetch)),
        KeyCode::Char('/') => Some(AppAction::Search(SearchAction::Open)),
        KeyCode::Char('n') => Some(AppAction::Search(SearchAction::NextResult)),
        KeyCode::Char('N') => Some(AppAction::Search(SearchAction::PreviousResult)),
        KeyCode::Char('F') => Some(AppAction::Filter(FilterAction::Open)),
        KeyCode::Char('B') => Some(AppAction::Git(GitAction::OpenBlame)),
        KeyCode::Char('x') => Some(AppAction::Git(GitAction::CherryPick)),
        KeyCode::Char('R') => Some(AppAction::Git(GitAction::ResetPrompt)),
        KeyCode::Char('I') => Some(AppAction::Git(GitAction::InteractiveRebase)),
        KeyCode::Char('Z') => Some(AppAction::Git(GitAction::UndoLastOperation)),
        KeyCode::Char('t') => Some(AppAction::Git(GitAction::CreateTag)),
        KeyCode::Char('T') => Some(AppAction::Git(GitAction::DeleteTag)),
        KeyCode::Char('C') => Some(AppAction::Git(GitAction::CompareSelectedWithHead)),
        KeyCode::Char('X') => Some(AppAction::Git(GitAction::BisectStart)),
        KeyCode::Char('i') => Some(AppAction::Git(GitAction::RepositoryInsights)),
        KeyCode::Char('[') if state.ui.is_bisecting => Some(AppAction::Git(GitAction::BisectGood)),
        KeyCode::Char(']') if state.ui.is_bisecting => Some(AppAction::Git(GitAction::BisectBad)),
        KeyCode::Char('\\') if state.ui.is_bisecting => {
            Some(AppAction::Git(GitAction::BisectReset))
        }
        KeyCode::Char('A') if state.ui.is_merging => Some(AppAction::Git(GitAction::AbortMerge)),
        KeyCode::Char('L') => Some(AppAction::LoadMoreHistory),
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),
        KeyCode::Char('r') => Some(AppAction::Refresh),
        KeyCode::Char('y') => Some(AppAction::CopyPanelContent),
        KeyCode::Tab => Some(AppAction::Navigation(NavigationAction::SwitchPanel)),
        KeyCode::Char('M') => Some(AppAction::SwitchBottomMode),
        _ => None,
    }
}

fn map_branches_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    map_branches_global_key(key).or_else(|| map_branches_section_key(key, state))
}

fn map_branches_global_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Tab => Some(AppAction::Branch(BranchAction::NextSection)),
        KeyCode::BackTab => Some(AppAction::Branch(BranchAction::PrevSection)),
        KeyCode::Char('q') => Some(AppAction::Quit),
        KeyCode::Char('y') => Some(AppAction::CopyPanelContent),
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),
        KeyCode::Char('P') => Some(AppAction::Git(GitAction::Push)),
        _ => None,
    }
}

fn map_branches_section_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match state.branches_view_state.section {
        BranchesSection::Branches => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Enter => Some(AppAction::Branch(BranchAction::Checkout)),
            KeyCode::Char('n') => Some(AppAction::Branch(BranchAction::Create)),
            KeyCode::Char('d') => Some(AppAction::Branch(BranchAction::Delete)),
            KeyCode::Char('r') => Some(AppAction::Branch(BranchAction::Rename)),
            KeyCode::Char('R') => Some(AppAction::Branch(BranchAction::ToggleRemote)),
            KeyCode::Char('m') => Some(AppAction::Git(GitAction::MergePrompt)),
            KeyCode::Char('e') => Some(AppAction::Git(GitAction::RebasePrompt)),
            _ => None,
        },
        BranchesSection::Worktrees => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Enter => Some(AppAction::Branch(BranchAction::WorktreeSwitch)),
            KeyCode::Char('n') => Some(AppAction::Branch(BranchAction::WorktreeCreate)),
            KeyCode::Char('d') => Some(AppAction::Branch(BranchAction::WorktreeRemove)),
            _ => None,
        },
        BranchesSection::Stashes => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Char('l') | KeyCode::Right => {
                Some(AppAction::Branch(BranchAction::StashFileNext))
            }
            KeyCode::Char('h') | KeyCode::Left => {
                Some(AppAction::Branch(BranchAction::StashFilePrev))
            }
            KeyCode::Char('a') => Some(AppAction::Branch(BranchAction::StashApply)),
            KeyCode::Char('p') => Some(AppAction::Branch(BranchAction::StashPop)),
            KeyCode::Char('d') => Some(AppAction::Branch(BranchAction::StashDrop)),
            KeyCode::Char('s') => Some(AppAction::Branch(BranchAction::StashSave)),
            KeyCode::Char('J') => {
                Some(AppAction::Navigation(NavigationAction::ScrollStashDiffDown))
            }
            KeyCode::Char('K') => Some(AppAction::Navigation(NavigationAction::ScrollStashDiffUp)),
            _ => None,
        },
    }
}

fn map_staging_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    map_staging_ctrl_key(key)
        .or_else(|| map_staging_global_key(key, state))
        .or_else(|| map_staging_focus_key(key, state))
}

fn map_staging_ctrl_key(key: KeyEvent) -> Option<AppAction> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }

    match key.code {
        KeyCode::Char('s') => Some(AppAction::Staging(StagingAction::StashUnstagedFiles)),
        KeyCode::Char('p') => Some(AppAction::Git(GitAction::ForcePush)),
        _ => None,
    }
}

fn map_staging_global_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('q') => Some(AppAction::Quit),
        KeyCode::Char('r') => Some(AppAction::Refresh),
        KeyCode::Char('y') => Some(AppAction::CopyPanelContent),
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),
        KeyCode::Char('P') => Some(AppAction::Git(GitAction::Push)),
        KeyCode::Char('A') if state.ui.is_merging => Some(AppAction::Git(GitAction::AbortMerge)),
        _ => None,
    }
}

fn map_staging_focus_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match state.staging_state.focus {
        StagingFocus::Unstaged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Char(' ') => Some(AppAction::Staging(StagingAction::FocusDiff)),
            KeyCode::Char('s') | KeyCode::Enter => {
                Some(AppAction::Staging(StagingAction::StageFile))
            }
            KeyCode::Char('S') => Some(AppAction::Staging(StagingAction::StashSelectedFile)),
            KeyCode::Char('a') => Some(AppAction::Staging(StagingAction::StageAll)),
            KeyCode::Char('d') => Some(AppAction::Staging(StagingAction::DiscardFile)),
            KeyCode::Char('D') => Some(AppAction::Staging(StagingAction::DiscardAll)),
            KeyCode::Tab => Some(AppAction::Staging(StagingAction::SwitchFocus)),
            KeyCode::Char('c') => Some(AppAction::Staging(StagingAction::StartCommitMessage)),
            KeyCode::Char('A') if !state.ui.is_merging => {
                Some(AppAction::Git(GitAction::AmendCommit))
            }
            KeyCode::Char('e') => Some(AppAction::Git(GitAction::OpenExternalDiff)),
            _ => None,
        },
        StagingFocus::Staged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Char(' ') => Some(AppAction::Staging(StagingAction::FocusDiff)),
            KeyCode::Char('u') | KeyCode::Enter => {
                Some(AppAction::Staging(StagingAction::UnstageFile))
            }
            KeyCode::Char('U') => Some(AppAction::Staging(StagingAction::UnstageAll)),
            KeyCode::Tab => Some(AppAction::Staging(StagingAction::SwitchFocus)),
            KeyCode::Char('c') => Some(AppAction::Staging(StagingAction::StartCommitMessage)),
            KeyCode::Char('A') if !state.ui.is_merging => {
                Some(AppAction::Git(GitAction::AmendCommit))
            }
            KeyCode::Char('e') => Some(AppAction::Git(GitAction::OpenExternalDiff)),
            _ => None,
        },
        StagingFocus::Diff => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffUp))
            }
            KeyCode::Char('h') | KeyCode::Left => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffLeft))
            }
            KeyCode::Char('l') | KeyCode::Right => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffRight))
            }
            KeyCode::Tab | KeyCode::Esc => Some(AppAction::Staging(StagingAction::SwitchFocus)),
            KeyCode::Char('c') => Some(AppAction::Staging(StagingAction::StartCommitMessage)),
            KeyCode::Char('A') if !state.ui.is_merging => {
                Some(AppAction::Git(GitAction::AmendCommit))
            }
            KeyCode::Char('v') => Some(AppAction::ToggleDiffViewMode),
            KeyCode::Char('e') => Some(AppAction::Git(GitAction::OpenExternalDiff)),
            KeyCode::Char('n') => Some(AppAction::Navigation(NavigationAction::NextDiffHunk)),
            KeyCode::Char('N') => Some(AppAction::Navigation(NavigationAction::PreviousDiffHunk)),
            KeyCode::Char('s') if state.staging_state.last_file_focus == StagingFocus::Unstaged => {
                Some(AppAction::Staging(StagingAction::StageHunk))
            }
            KeyCode::Char('S') if state.staging_state.last_file_focus == StagingFocus::Unstaged => {
                Some(AppAction::Staging(StagingAction::StageLine))
            }
            KeyCode::Char('u') if state.staging_state.last_file_focus == StagingFocus::Staged => {
                Some(AppAction::Staging(StagingAction::UnstageHunk))
            }
            KeyCode::Char('U') if state.staging_state.last_file_focus == StagingFocus::Staged => {
                Some(AppAction::Staging(StagingAction::UnstageLine))
            }
            _ => None,
        },
        StagingFocus::CommitMessage => None,
    }
}

fn map_blame_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(AppAction::Git(GitAction::CloseBlame)),
        KeyCode::Char('j') | KeyCode::Down => {
            Some(AppAction::Navigation(NavigationAction::MoveDown))
        }
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::Navigation(NavigationAction::MoveUp)),
        KeyCode::Char('g') | KeyCode::Home => Some(AppAction::Navigation(NavigationAction::GoTop)),
        KeyCode::Char('G') | KeyCode::End => {
            Some(AppAction::Navigation(NavigationAction::GoBottom))
        }
        KeyCode::PageUp => Some(AppAction::Navigation(NavigationAction::PageUp)),
        KeyCode::PageDown => Some(AppAction::Navigation(NavigationAction::PageDown)),
        KeyCode::Enter => Some(AppAction::Git(GitAction::JumpToBlameCommit)),
        KeyCode::Char('y') => Some(AppAction::CopyPanelContent),
        _ => None,
    }
}

fn map_conflicts_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    map_conflicts_editing_key(key, state).or_else(|| map_conflicts_navigation_key(key, state))
}

fn map_conflicts_editing_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    let is_editing = state
        .conflicts_state
        .as_ref()
        .is_some_and(|conflicts| conflicts.is_editing);
    if !is_editing {
        return None;
    }

    match key.code {
        KeyCode::Esc => Some(AppAction::Conflict(ConflictAction::StopEditing)),
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Conflict(ConflictAction::ConfirmEdit))
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Conflict(ConflictAction::ConfirmEdit))
        }
        KeyCode::Char(c) => Some(AppAction::Conflict(ConflictAction::EditInsertChar(c))),
        KeyCode::Backspace => Some(AppAction::Conflict(ConflictAction::EditBackspace)),
        KeyCode::Delete => Some(AppAction::Conflict(ConflictAction::EditDelete)),
        KeyCode::Enter => Some(AppAction::Conflict(ConflictAction::EditNewline)),
        KeyCode::Up => Some(AppAction::Conflict(ConflictAction::EditCursorUp)),
        KeyCode::Down => Some(AppAction::Conflict(ConflictAction::EditCursorDown)),
        KeyCode::Left => Some(AppAction::Conflict(ConflictAction::EditCursorLeft)),
        KeyCode::Right => Some(AppAction::Conflict(ConflictAction::EditCursorRight)),
        _ => None,
    }
}

fn map_conflicts_navigation_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    use crate::git::conflict::ConflictResolutionMode;

    let conflicts_state = state.conflicts_state.as_ref();
    let panel_focus = conflicts_state.map(|conflicts| conflicts.panel_focus);
    let resolution_mode = conflicts_state
        .map(|conflicts| conflicts.resolution_mode)
        .unwrap_or(ConflictResolutionMode::Block);

    match key.code {
        KeyCode::Tab | KeyCode::BackTab => Some(AppAction::Conflict(ConflictAction::SwitchPanel)),
        KeyCode::Char('j') | KeyCode::Down => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::NextFile))
            }
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    ConflictResolutionMode::File => {
                        Some(AppAction::Conflict(ConflictAction::NextFile))
                    }
                    ConflictResolutionMode::Line => {
                        Some(AppAction::Conflict(ConflictAction::LineDown))
                    }
                    ConflictResolutionMode::Block => {
                        Some(AppAction::Conflict(ConflictAction::NextSection))
                    }
                }
            }
            Some(ConflictPanelFocus::ResultPanel) => {
                Some(AppAction::Conflict(ConflictAction::ResultScrollDown))
            }
            _ => None,
        },
        KeyCode::Char('k') | KeyCode::Up => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::PreviousFile))
            }
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    ConflictResolutionMode::File => {
                        Some(AppAction::Conflict(ConflictAction::PreviousFile))
                    }
                    ConflictResolutionMode::Line => {
                        Some(AppAction::Conflict(ConflictAction::LineUp))
                    }
                    ConflictResolutionMode::Block => {
                        Some(AppAction::Conflict(ConflictAction::PreviousSection))
                    }
                }
            }
            Some(ConflictPanelFocus::ResultPanel) => {
                Some(AppAction::Conflict(ConflictAction::ResultScrollUp))
            }
            _ => None,
        },
        KeyCode::Char('o') | KeyCode::Left => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::AcceptOursFile))
            }
            _ => None,
        },
        KeyCode::Char('t') | KeyCode::Right => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::AcceptTheirsFile))
            }
            _ => None,
        },
        KeyCode::Char('r') => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::MarkResolved))
            }
            _ => None,
        },
        KeyCode::Char('b')
            if matches!(
                panel_focus,
                Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel)
            ) && resolution_mode == ConflictResolutionMode::Block =>
        {
            Some(AppAction::Conflict(ConflictAction::AcceptBoth))
        }
        KeyCode::Char('i' | 'e') if panel_focus == Some(ConflictPanelFocus::ResultPanel) => {
            Some(AppAction::Conflict(ConflictAction::StartEditing))
        }
        KeyCode::Char('F') => Some(AppAction::Conflict(ConflictAction::SetModeFile)),
        KeyCode::Char('B') => Some(AppAction::Conflict(ConflictAction::SetModeBlock)),
        KeyCode::Char('L') => Some(AppAction::Conflict(ConflictAction::SetModeLine)),
        KeyCode::Char(' ') => match panel_focus {
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    ConflictResolutionMode::Block => {
                        Some(AppAction::Conflict(ConflictAction::EnterResolve))
                    }
                    ConflictResolutionMode::Line => {
                        Some(AppAction::Conflict(ConflictAction::ToggleLine))
                    }
                    ConflictResolutionMode::File => None,
                }
            }
            _ => None,
        },
        KeyCode::Enter => match panel_focus {
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    ConflictResolutionMode::File => {
                        Some(AppAction::Conflict(ConflictAction::EnterResolve))
                    }
                    ConflictResolutionMode::Block | ConflictResolutionMode::Line => {
                        Some(AppAction::Conflict(ConflictAction::MarkResolved))
                    }
                }
            }
            _ => None,
        },
        KeyCode::Char('V') => Some(AppAction::Conflict(ConflictAction::FinalizeMerge)),
        KeyCode::Char('q') | KeyCode::Esc => Some(AppAction::Conflict(ConflictAction::LeaveView)),
        KeyCode::Char('A') => Some(AppAction::Conflict(ConflictAction::AbortMerge)),
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::state::ConflictPanelFocus;
    use ratatui::layout::Rect;

    fn create_test_state() -> AppState {
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

        let git_repo = GitRepo::open(temp_dir.path().to_string_lossy().as_ref()).unwrap();
        let mut state =
            AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();
        state.update_screen_area(Rect::new(0, 0, 120, 40));
        state
    }

    #[test]
    fn test_input_priority_confirmation_over_view_keys() {
        let mut state = create_test_state();
        state.open_confirmation(crate::ui::confirm_dialog::ConfirmAction::DiscardAll);

        let action = map_key(
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
            &state,
        );

        assert_eq!(action, None);
    }

    #[test]
    fn test_input_priority_search_over_view_switch() {
        let mut state = create_test_state();
        state.search_state.is_active = true;

        let action = map_key(
            KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE),
            &state,
        );

        assert_eq!(
            action,
            Some(AppAction::Search(SearchAction::InsertChar('3')))
        );
    }

    #[test]
    fn test_input_priority_branches_input_over_view_switch() {
        let mut state = create_test_state();
        state.view_mode = ViewMode::Branches;
        state.branches_view_state.focus = BranchesFocus::Input;

        let action = map_key(
            KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE),
            &state,
        );

        assert_eq!(action, Some(AppAction::Edit(EditAction::InsertChar('1'))));
    }

    #[test]
    fn test_input_priority_conflict_editing_over_help_and_leave_view() {
        let mut state = create_test_state();
        state.view_mode = ViewMode::Conflicts;
        state.conflicts_state = Some(crate::state::ConflictsState::new(
            Vec::new(),
            "merge test".to_string(),
            "main".to_string(),
            "feature".to_string(),
        ));
        let conflicts = state.conflicts_state.as_mut().unwrap();
        conflicts.is_editing = true;
        conflicts.panel_focus = ConflictPanelFocus::ResultPanel;

        let action = map_key(
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
            &state,
        );

        assert_eq!(
            action,
            Some(AppAction::Conflict(ConflictAction::EditInsertChar('?')))
        );
    }

    #[test]
    fn test_search_command_left_moves_to_start() {
        let mut state = create_test_state();
        state.search_state.is_active = true;

        let action = map_key(KeyEvent::new(KeyCode::Left, KeyModifiers::SUPER), &state);

        assert_eq!(
            action,
            Some(AppAction::Search(SearchAction::Edit(
                EditAction::CursorHome
            )))
        );
    }

    #[test]
    fn test_filter_option_shift_left_selects_previous_word() {
        let mut state = create_test_state();
        state.filters.filter_popup.is_open = true;

        let action = map_key(
            KeyEvent::new(KeyCode::Left, KeyModifiers::ALT | KeyModifiers::SHIFT),
            &state,
        );

        assert_eq!(
            action,
            Some(AppAction::Filter(FilterAction::Edit(
                EditAction::SelectWordLeft
            )))
        );
    }

    #[test]
    fn test_search_command_z_undoes_editing() {
        let mut state = create_test_state();
        state.search_state.is_active = true;

        let action = map_key(
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::SUPER),
            &state,
        );

        assert_eq!(
            action,
            Some(AppAction::Search(SearchAction::Edit(EditAction::Undo)))
        );
    }

    #[test]
    fn test_w_opens_worktree_selector() {
        let state = create_test_state();
        let action = map_key(
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
            &state,
        );

        assert_eq!(action, Some(AppAction::Branch(BranchAction::OpenWorktrees)));
    }

    #[test]
    fn project_tree_arrows_expand_and_collapse_directories() {
        let mut state = create_test_state();
        state.view_mode = ViewMode::ProjectTree;

        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), &state),
            Some(AppAction::ProjectTree(ProjectTreeAction::ExpandSelected))
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &state),
            Some(AppAction::ProjectTree(ProjectTreeAction::CollapseSelected))
        );
    }

    #[test]
    fn project_tree_search_captures_text_and_confirmation() {
        let mut state = create_test_state();
        state.view_mode = ViewMode::ProjectTree;
        state.project_tree_state.open_search();

        assert_eq!(
            map_key(
                KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
                &state,
            ),
            Some(AppAction::ProjectTree(ProjectTreeAction::EditSearch(
                EditAction::InsertChar('4')
            )))
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &state),
            Some(AppAction::ProjectTree(ProjectTreeAction::ConfirmSearch))
        );
    }
}
