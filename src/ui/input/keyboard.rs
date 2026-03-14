use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::state::action::{
    BranchAction, ConflictAction, EditAction, FilterAction, GitAction, NavigationAction,
    SearchAction, StagingAction,
};
use crate::state::{
    AppAction, AppState, BranchesFocus, BranchesSection, ConflictPanelFocus, FocusPanel,
    StagingFocus, ViewMode,
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
        || state.ui.pending_confirmation.is_some()
}

fn has_blocking_text_input(state: &AppState) -> bool {
    state.search_state.is_active
        || state.filters.filter_popup.is_open
        || (state.view_mode == ViewMode::Staging
            && state.staging_state.focus == StagingFocus::CommitMessage)
        || (state.view_mode == ViewMode::Branches
            && state.branches_view_state.focus == BranchesFocus::Input)
        || (state.view_mode == ViewMode::Conflicts
            && state
                .conflicts_state
                .as_ref()
                .is_some_and(|conflicts| conflicts.is_editing))
}

fn map_modal_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
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

    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.focus == BranchesFocus::Input
    {
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
        KeyCode::Char('4') if state.conflicts_state.is_some() => {
            Some(AppAction::SwitchView(ViewMode::Conflicts))
        }
        _ => None,
    }
}

fn map_view_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match state.view_mode {
        ViewMode::Graph => map_graph_key(key, state),
        ViewMode::Staging => map_staging_key(key, state),
        ViewMode::Branches => map_branches_key(key, state),
        ViewMode::Conflicts => map_conflicts_key(key, state),
        ViewMode::Blame => map_blame_key(key),
        ViewMode::Help => None,
    }
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
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('s') => {
            Some(AppAction::ResetPickerSelectSoft)
        }
        KeyCode::Char('k') | KeyCode::Up | KeyCode::Char('h') => {
            Some(AppAction::ResetPickerSelectHard)
        }
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
    match key.code {
        KeyCode::Esc => Some(AppAction::Search(SearchAction::Close)),
        KeyCode::Enter => Some(AppAction::Search(SearchAction::Execute)),
        KeyCode::Down => Some(AppAction::Search(SearchAction::NextResult)),
        KeyCode::Up => Some(AppAction::Search(SearchAction::PreviousResult)),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Search(SearchAction::NextResult))
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Search(SearchAction::PreviousResult))
        }
        KeyCode::Tab => Some(AppAction::Search(SearchAction::ChangeType)),
        KeyCode::Char(c) => Some(AppAction::Search(SearchAction::InsertChar(c))),
        KeyCode::Backspace => Some(AppAction::Search(SearchAction::DeleteChar)),
        _ => None,
    }
}

fn map_filter_popup_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Esc => Some(AppAction::Filter(FilterAction::Close)),
        KeyCode::Enter => Some(AppAction::Filter(FilterAction::Apply)),
        KeyCode::Tab | KeyCode::Down => Some(AppAction::Filter(FilterAction::NextField)),
        KeyCode::BackTab | KeyCode::Up => Some(AppAction::Filter(FilterAction::PreviousField)),
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Filter(FilterAction::Clear))
        }
        KeyCode::Char(c) => Some(AppAction::Filter(FilterAction::InsertChar(c))),
        KeyCode::Backspace => Some(AppAction::Filter(FilterAction::DeleteChar)),
        _ => None,
    }
}

fn map_staging_commit_input_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Enter => Some(AppAction::Staging(StagingAction::ConfirmCommit)),
        KeyCode::Esc => Some(AppAction::Staging(StagingAction::CancelCommit)),
        KeyCode::Char(c) => Some(AppAction::Edit(EditAction::InsertChar(c))),
        KeyCode::Backspace => Some(AppAction::Edit(EditAction::DeleteCharBefore)),
        KeyCode::Left => Some(AppAction::Edit(EditAction::CursorLeft)),
        KeyCode::Right => Some(AppAction::Edit(EditAction::CursorRight)),
        _ => None,
    }
}

fn map_branches_input_key(key: KeyEvent) -> Option<AppAction> {
    match key.code {
        KeyCode::Enter => Some(AppAction::Branch(BranchAction::ConfirmInput)),
        KeyCode::Esc => Some(AppAction::Branch(BranchAction::CancelInput)),
        KeyCode::Char(c) => Some(AppAction::Edit(EditAction::InsertChar(c))),
        KeyCode::Backspace => Some(AppAction::Edit(EditAction::DeleteCharBefore)),
        KeyCode::Left => Some(AppAction::Edit(EditAction::CursorLeft)),
        KeyCode::Right => Some(AppAction::Edit(EditAction::CursorRight)),
        _ => None,
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
            _ => None,
        },
        BranchesSection::Worktrees => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
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
    map_staging_global_key(key, state).or_else(|| map_staging_focus_key(key, state))
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
            _ if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('S') =>
            {
                Some(AppAction::Staging(StagingAction::StashUnstagedFiles))
            }
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
}
