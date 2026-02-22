//! Handler pour les actions d'édition de texte.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::EditAction;
use crate::state::AppState;

/// Handler pour les opérations d'édition de texte.
pub struct EditHandler;

impl ActionHandler for EditHandler {
    type Action = EditAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: EditAction) -> Result<()> {
        match action {
            EditAction::InsertChar(c) => handle_insert_char(ctx.state, c),
            EditAction::DeleteCharBefore => handle_delete_char_before(ctx.state),
            EditAction::DeleteCharAfter => handle_delete_char_after(ctx.state),
            EditAction::CursorLeft => handle_cursor_left(ctx.state),
            EditAction::CursorRight => handle_cursor_right(ctx.state),
            EditAction::CursorHome => handle_cursor_home(ctx.state),
            EditAction::CursorEnd => handle_cursor_end(ctx.state),
            EditAction::NewLine => handle_new_line(ctx.state),
        }
    }
}

fn handle_insert_char(state: &mut AppState, c: char) -> Result<()> {
    if state.staging_state.is_committing {
        let pos = state.staging_state.cursor_position;
        if pos <= state.staging_state.commit_message.len() {
            state.staging_state.commit_message.insert(pos, c);
            state.staging_state.cursor_position += 1;
        }
    }
    Ok(())
}

fn handle_delete_char_before(state: &mut AppState) -> Result<()> {
    if state.staging_state.is_committing {
        let pos = state.staging_state.cursor_position;
        if pos > 0 && pos <= state.staging_state.commit_message.len() {
            state.staging_state.commit_message.remove(pos - 1);
            state.staging_state.cursor_position -= 1;
        }
    }
    Ok(())
}

fn handle_delete_char_after(state: &mut AppState) -> Result<()> {
    if state.staging_state.is_committing {
        let pos = state.staging_state.cursor_position;
        if pos < state.staging_state.commit_message.len() {
            state.staging_state.commit_message.remove(pos);
        }
    }
    Ok(())
}

fn handle_cursor_left(state: &mut AppState) -> Result<()> {
    if state.staging_state.is_committing && state.staging_state.cursor_position > 0 {
        state.staging_state.cursor_position -= 1;
    }
    Ok(())
}

fn handle_cursor_right(state: &mut AppState) -> Result<()> {
    if state.staging_state.is_committing {
        let len = state.staging_state.commit_message.len();
        if state.staging_state.cursor_position < len {
            state.staging_state.cursor_position += 1;
        }
    }
    Ok(())
}

fn handle_cursor_home(state: &mut AppState) -> Result<()> {
    if state.staging_state.is_committing {
        state.staging_state.cursor_position = 0;
    }
    Ok(())
}

fn handle_cursor_end(state: &mut AppState) -> Result<()> {
    if state.staging_state.is_committing {
        state.staging_state.cursor_position = state.staging_state.commit_message.len();
    }
    Ok(())
}

fn handle_new_line(state: &mut AppState) -> Result<()> {
    if state.staging_state.is_committing {
        let pos = state.staging_state.cursor_position;
        if pos <= state.staging_state.commit_message.len() {
            state.staging_state.commit_message.insert(pos, '\n');
            state.staging_state.cursor_position += 1;
        }
    }
    Ok(())
}
