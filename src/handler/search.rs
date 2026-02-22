//! Handler pour les actions de recherche.

use crate::error::Result;
use crate::state::AppState;
use crate::state::action::SearchAction;
use super::traits::{ActionHandler, HandlerContext};

/// Handler pour les opérations de recherche.
pub struct SearchHandler;

impl ActionHandler for SearchHandler {
    type Action = SearchAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: SearchAction) -> Result<()> {
        match action {
            SearchAction::Open => handle_open(ctx.state),
            SearchAction::Close => handle_close(ctx.state),
            SearchAction::InsertChar(c) => handle_insert_char(ctx.state, c),
            SearchAction::DeleteChar => handle_delete_char(ctx.state),
            SearchAction::NextResult => handle_next_result(ctx.state),
            SearchAction::PreviousResult => handle_previous_result(ctx.state),
            SearchAction::ChangeType => handle_change_type(ctx.state),
            SearchAction::Execute => handle_execute(ctx.state),
        }
    }
}

fn handle_open(state: &mut AppState) -> Result<()> {
    state.search_state.is_active = true;
    state.search_state.query.clear();
    state.search_state.results.clear();
    state.search_state.current_result = 0;
    Ok(())
}

fn handle_close(state: &mut AppState) -> Result<()> {
    state.search_state.is_active = false;
    state.search_state.query.clear();
    state.search_state.results.clear();
    Ok(())
}

fn handle_insert_char(state: &mut AppState, c: char) -> Result<()> {
    state.search_state.query.push(c);
    state.search_state.cursor += 1;
    // Exécuter la recherche incrémentale automatiquement
    handle_execute(state)?;
    Ok(())
}

fn handle_delete_char(state: &mut AppState) -> Result<()> {
    if state.search_state.cursor > 0 && !state.search_state.query.is_empty() {
        state.search_state.cursor -= 1;
        state.search_state.query.remove(state.search_state.cursor);
        // Exécuter la recherche incrémentale automatiquement
        handle_execute(state)?;
    }
    Ok(())
}

fn handle_next_result(state: &mut AppState) -> Result<()> {
    if !state.search_state.results.is_empty() {
        state.search_state.current_result = (state.search_state.current_result + 1) % state.search_state.results.len();
        // Naviguer vers le résultat sélectionné
        if let Some(&index) = state.search_state.results.get(state.search_state.current_result) {
            if index < state.graph.len() {
                state.selected_index = index;
                state.graph_state.select(Some(index * 2));
                state.sync_legacy_selection();
            }
        }
    }
    Ok(())
}

fn handle_previous_result(state: &mut AppState) -> Result<()> {
    if !state.search_state.results.is_empty() {
        let len = state.search_state.results.len();
        state.search_state.current_result = (state.search_state.current_result + len - 1) % len;
        // Naviguer vers le résultat sélectionné
        if let Some(&index) = state.search_state.results.get(state.search_state.current_result) {
            if index < state.graph.len() {
                state.selected_index = index;
                state.graph_state.select(Some(index * 2));
                state.sync_legacy_selection();
            }
        }
    }
    Ok(())
}

fn handle_change_type(state: &mut AppState) -> Result<()> {
    use crate::git::search::SearchType;

    state.search_state.search_type = match state.search_state.search_type {
        SearchType::Message => SearchType::Author,
        SearchType::Author => SearchType::Hash,
        SearchType::Hash => SearchType::Message,
    };
    Ok(())
}

fn handle_execute(state: &mut AppState) -> Result<()> {
    if state.search_state.query.is_empty() {
        return Ok(());
    }

    let query = state.search_state.query.clone();
    let search_type = state.search_state.search_type;

    // Utiliser filter_commits de git::search
    let results = crate::git::search::filter_commits(&state.graph, &query, search_type);

    state.search_state.results = results;
    state.search_state.current_result = 0;

    if !state.search_state.results.is_empty() {
        // Naviguer vers le premier résultat
        handle_next_result(state)?;
        state.set_flash_message(format!("{} résultats trouvés", state.search_state.results.len()));
    } else {
        state.set_flash_message("Aucun résultat".to_string());
    }

    Ok(())
}
