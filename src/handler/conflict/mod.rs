//! Handler pour les actions de resolution de conflits.

mod edit;
mod modes;
mod navigation;
mod shared;

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::ConflictAction;

pub struct ConflictHandler;

impl ActionHandler for ConflictHandler {
    type Action = ConflictAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: ConflictAction) -> Result<()> {
        match action {
            ConflictAction::PreviousFile => navigation::handle_previous_file(ctx.state),
            ConflictAction::NextFile => navigation::handle_next_file(ctx.state),
            ConflictAction::PreviousSection => navigation::handle_previous_section(ctx.state),
            ConflictAction::NextSection => navigation::handle_next_section(ctx.state),
            ConflictAction::SwitchPanel => navigation::handle_switch_panel(ctx.state),
            ConflictAction::PreviousPanel => navigation::handle_previous_panel(ctx.state),
            ConflictAction::AcceptOursFile => navigation::handle_accept_ours_file(ctx.state),
            ConflictAction::AcceptTheirsFile => navigation::handle_accept_theirs_file(ctx.state),
            ConflictAction::AcceptOursBlock => navigation::handle_accept_ours_block(ctx.state),
            ConflictAction::AcceptTheirsBlock => navigation::handle_accept_theirs_block(ctx.state),
            ConflictAction::AcceptBoth => navigation::handle_accept_both(ctx.state),
            ConflictAction::StartEdit => edit::handle_start_edit(ctx.state),
            ConflictAction::ConfirmEdit => edit::handle_confirm_edit(ctx.state),
            ConflictAction::CancelEdit => edit::handle_cancel_edit(ctx.state),
            ConflictAction::MarkResolved => navigation::handle_mark_resolved(ctx.state),
            ConflictAction::FinalizeMerge => navigation::handle_finalize_merge(ctx.state),
            ConflictAction::AbortMerge => navigation::handle_abort_merge(ctx.state),
            ConflictAction::SetModeFile => modes::handle_set_mode_file(ctx.state),
            ConflictAction::SetModeBlock => modes::handle_set_mode_block(ctx.state),
            ConflictAction::SetModeLine => modes::handle_set_mode_line(ctx.state),
            ConflictAction::ToggleLine => modes::handle_toggle_line(ctx.state),
            ConflictAction::LineDown => modes::handle_line_down(ctx.state),
            ConflictAction::LineUp => modes::handle_line_up(ctx.state),
            ConflictAction::ResultScrollDown => modes::handle_result_scroll_down(ctx.state),
            ConflictAction::ResultScrollUp => modes::handle_result_scroll_up(ctx.state),
            ConflictAction::StartEditing => modes::handle_start_editing(ctx.state),
            ConflictAction::StopEditing => modes::handle_stop_editing(ctx.state),
            ConflictAction::EditInsertChar(c) => modes::handle_edit_insert_char(ctx.state, c),
            ConflictAction::EditInsertText(text) => {
                modes::handle_edit_insert_text(ctx.state, &text)
            }
            ConflictAction::EditBackspace => modes::handle_edit_backspace(ctx.state),
            ConflictAction::EditDelete => modes::handle_edit_delete(ctx.state),
            ConflictAction::EditCursorUp => modes::handle_edit_cursor_up(ctx.state),
            ConflictAction::EditCursorDown => modes::handle_edit_cursor_down(ctx.state),
            ConflictAction::EditCursorLeft => modes::handle_edit_cursor_left(ctx.state),
            ConflictAction::EditCursorRight => modes::handle_edit_cursor_right(ctx.state),
            ConflictAction::EditNewline => modes::handle_edit_newline(ctx.state),
            ConflictAction::LeaveView => modes::handle_leave_view(ctx.state),
            ConflictAction::EnterResolve => modes::handle_enter_resolve(ctx.state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::conflict::{ConflictFile, ConflictType};
    use crate::git::tests::test_utils::{commit_file, create_test_repo};
    use crate::state::{AppState, ConflictsState};

    fn state_with_conflicts(files: Vec<ConflictFile>) -> (tempfile::TempDir, AppState) {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "nested/file.txt", "base\n", "Initial");
        let mut state = AppState::new(
            crate::git::GitRepo { repo },
            temp.path().display().to_string(),
        )
        .unwrap();
        state.open_conflicts(ConflictsState::new(
            files,
            "merge".into(),
            "main".into(),
            "feature".into(),
        ));
        (temp, state)
    }

    #[test]
    fn test_empty_conflicts_navigation_does_not_panic() {
        let (_temp, mut state) = state_with_conflicts(Vec::new());
        navigation::handle_next_section(&mut state).unwrap();
        modes::handle_line_down(&mut state).unwrap();
        modes::handle_line_up(&mut state).unwrap();
        modes::handle_start_editing(&mut state).unwrap();
        assert!(!state.conflicts_state.as_ref().unwrap().is_editing);
    }

    #[test]
    fn test_manual_conflict_edit_writes_into_selected_repository() {
        let file = ConflictFile {
            path: "nested/file.txt".into(),
            conflicts: Vec::new(),
            is_resolved: false,
            conflict_type: ConflictType::BothModified,
        };
        let (temp, mut state) = state_with_conflicts(vec![file]);
        edit::handle_start_edit(&mut state).unwrap();
        modes::handle_edit_insert_char(&mut state, 'é').unwrap();
        edit::handle_confirm_edit(&mut state).unwrap();

        assert_eq!(
            std::fs::read_to_string(temp.path().join("nested/file.txt")).unwrap(),
            "ébase\n"
        );
        let index = state.repo.repo.index().unwrap();
        let entry = index
            .get_path(std::path::Path::new("nested/file.txt"), 0)
            .unwrap();
        assert_eq!(
            state.repo.repo.find_blob(entry.id).unwrap().content(),
            "ébase\n".as_bytes()
        );
        assert!(state.conflicts_state.as_ref().unwrap().all_files[0].is_resolved);
        let conflicts = state.conflicts_state.as_ref().unwrap();
        let preview = crate::git::conflict::generate_resolved_content_with_source(
            &conflicts.all_files[0],
            conflicts.resolution_mode,
        )
        .into_iter()
        .map(|line| line.content)
        .collect::<Vec<_>>()
        .join("\n");
        assert_eq!(preview, "ébase\n");
    }
    #[test]
    fn audit_manual_edit_refuses_to_overwrite_external_changes() {
        let file = ConflictFile {
            path: "nested/file.txt".into(),
            conflicts: Vec::new(),
            is_resolved: false,
            conflict_type: ConflictType::BothModified,
        };
        let (temp, mut state) = state_with_conflicts(vec![file]);
        edit::handle_start_edit(&mut state).unwrap();
        modes::handle_edit_insert_char(&mut state, 'é').unwrap();
        std::fs::write(
            temp.path().join("nested/file.txt"),
            "modification externe\n",
        )
        .unwrap();
        edit::handle_confirm_edit(&mut state).unwrap();
        assert_eq!(
            std::fs::read_to_string(temp.path().join("nested/file.txt")).unwrap(),
            "modification externe\n"
        );
        assert!(state.conflicts_state.as_ref().unwrap().is_editing);
    }

    #[test]
    fn audit_manual_edit_preserves_crlf_when_splitting_and_joining_lines() {
        let file = ConflictFile {
            path: "nested/file.txt".into(),
            conflicts: Vec::new(),
            is_resolved: false,
            conflict_type: ConflictType::BothModified,
        };
        let (temp, mut state) = state_with_conflicts(vec![file]);
        std::fs::write(temp.path().join("nested/file.txt"), "é界\r\nfin\r\n").unwrap();
        edit::handle_start_edit(&mut state).unwrap();
        state.conflicts_state.as_mut().unwrap().edit_cursor_col = 1;
        modes::handle_edit_newline(&mut state).unwrap();
        assert_eq!(
            state
                .conflicts_state
                .as_ref()
                .unwrap()
                .edit_buffer
                .join("\n"),
            "é\r\n界\r\nfin\r\n"
        );
        modes::handle_edit_backspace(&mut state).unwrap();
        assert_eq!(
            state
                .conflicts_state
                .as_ref()
                .unwrap()
                .edit_buffer
                .join("\n"),
            "é界\r\nfin\r\n"
        );
        modes::handle_edit_cursor_right(&mut state).unwrap();
        modes::handle_edit_delete(&mut state).unwrap();
        assert_eq!(
            state
                .conflicts_state
                .as_ref()
                .unwrap()
                .edit_buffer
                .join("\n"),
            "é界fin\r\n"
        );
    }

    #[test]
    fn paste_into_conflict_editor_preserves_exact_unicode_and_line_endings() {
        let file = ConflictFile {
            path: "nested/file.txt".into(),
            conflicts: Vec::new(),
            is_resolved: false,
            conflict_type: ConflictType::BothModified,
        };
        let (temp, mut state) = state_with_conflicts(vec![file]);
        edit::handle_start_edit(&mut state).unwrap();
        state.conflicts_state.as_mut().unwrap().edit_cursor_col = 2;
        modes::handle_edit_insert_text(&mut state, "é界\r\npd\n").unwrap();
        assert!(state.conflicts_state.as_ref().unwrap().is_editing);
        assert_eq!(
            std::fs::read_to_string(temp.path().join("nested/file.txt")).unwrap(),
            "base\n"
        );
        edit::handle_confirm_edit(&mut state).unwrap();
        assert_eq!(
            std::fs::read_to_string(temp.path().join("nested/file.txt")).unwrap(),
            "baé界\r\npd\nse\n"
        );
        assert!(state
            .conflicts_state
            .as_ref()
            .unwrap()
            .edit_original
            .is_none());
        assert!(state
            .conflicts_state
            .as_ref()
            .unwrap()
            .edit_buffer
            .is_empty());
    }

    #[test]
    fn changing_a_line_starts_from_the_explicit_block_choice() {
        use crate::git::conflict::{
            ConflictResolution, ConflictResolutionMode, ConflictSection, LineLevelResolution,
        };
        use crate::state::ConflictPanelFocus;
        let file = ConflictFile {
            path: "nested/file.txt".into(),
            conflicts: vec![ConflictSection {
                context_before: vec![],
                ours: vec!["ours".into()],
                theirs: vec!["theirs".into()],
                context_after: vec![],
                resolution: Some(ConflictResolution::Theirs),
                line_level_resolution: Some(LineLevelResolution::new(1, 1)),
            }],
            is_resolved: false,
            conflict_type: ConflictType::BothModified,
        };
        let (_temp, mut state) = state_with_conflicts(vec![file]);
        let conflicts = state.conflicts_state.as_mut().unwrap();
        conflicts.resolution_mode = ConflictResolutionMode::Line;
        conflicts.panel_focus = ConflictPanelFocus::OursPanel;
        modes::handle_toggle_line(&mut state).unwrap();
        let file = &state.conflicts_state.as_ref().unwrap().all_files[0];
        let section = &file.conflicts[0];
        assert_eq!(section.resolution, None);
        let lines = section.line_level_resolution.as_ref().unwrap();
        assert_eq!(lines.ours_lines_included, [true]);
        assert_eq!(lines.theirs_lines_included, [true]);
        assert!(!crate::git::conflict::all_sections_resolved(
            file,
            ConflictResolutionMode::Block
        ));
    }

    #[test]
    fn previous_panel_is_the_inverse_of_next_panel() {
        let (_temp, mut state) = state_with_conflicts(Vec::new());
        let first = state.conflicts_state.as_ref().unwrap().panel_focus;
        navigation::handle_previous_panel(&mut state).unwrap();
        assert_eq!(
            state.conflicts_state.as_ref().unwrap().panel_focus,
            crate::state::ConflictPanelFocus::ResultPanel
        );
        navigation::handle_switch_panel(&mut state).unwrap();
        assert_eq!(state.conflicts_state.as_ref().unwrap().panel_focus, first);
    }
    #[cfg(unix)]
    #[test]
    fn conflict_editor_never_reads_or_overwrites_a_symlink_target_outside_the_repo() {
        use std::os::unix::fs::symlink;
        for linked_parent in [false, true] {
            let file = ConflictFile {
                path: "nested/file.txt".into(),
                conflicts: Vec::new(),
                is_resolved: false,
                conflict_type: ConflictType::BothModified,
            };
            let (temp, mut state) = state_with_conflicts(vec![file]);
            edit::handle_start_edit(&mut state).unwrap();
            modes::handle_edit_insert_char(&mut state, 'X').unwrap();
            let outside = tempfile::TempDir::new().unwrap();
            let sentinel = outside.path().join("file.txt");
            std::fs::write(&sentinel, "base\n").unwrap();
            if linked_parent {
                std::fs::rename(temp.path().join("nested"), temp.path().join("saved-nested"))
                    .unwrap();
                symlink(outside.path(), temp.path().join("nested")).unwrap();
            } else {
                std::fs::remove_file(temp.path().join("nested/file.txt")).unwrap();
                symlink(&sentinel, temp.path().join("nested/file.txt")).unwrap();
            }
            edit::handle_confirm_edit(&mut state).unwrap();
            assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), "base\n");
            assert!(state.conflicts_state.as_ref().unwrap().is_editing);
            assert!(modes::handle_start_editing(&mut state).is_err());
            let mut deleted = state.conflicts_state.as_ref().unwrap().all_files[0].clone();
            deleted.conflict_type = Some(ConflictType::DeletedByThem);
            assert!(crate::git::conflict::resolve_special_file(
                &state.repo.repo,
                &deleted,
                crate::git::conflict::ConflictResolution::Theirs
            )
            .is_err());
            assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), "base\n");
        }
    }
}
