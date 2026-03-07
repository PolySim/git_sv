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
