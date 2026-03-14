//! Handler pour les actions de recherche.

use super::traits::{ActionHandler, HandlerContext};
use crate::error::Result;
use crate::state::action::SearchAction;
use crate::state::AppState;
use crate::utils::flash_success;

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
    state.search_state.open();
    Ok(())
}

fn handle_close(state: &mut AppState) -> Result<()> {
    state.search_state.close();
    // Ne PAS effacer query et results pour permettre la navigation n/N après fermeture
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
        state.search_state.next_result();
        // Naviguer vers le résultat sélectionné
        if let Some(&index) = state
            .search_state
            .results
            .get(state.search_state.current_result)
        {
            if index < state.graph_view.len() {
                state.graph_view.select_commit(index);
                // Rafraîchir les fichiers du commit sélectionné
                state.refresh_commit_files();
                // Charger le diff du fichier si disponible
                if !state.graph_view.commit_files.is_empty() {
                    crate::handler::navigation::load_commit_file_diff(state);
                }
            }
        }
    }
    Ok(())
}

fn handle_previous_result(state: &mut AppState) -> Result<()> {
    if !state.search_state.results.is_empty() {
        state.search_state.previous_result();
        // Naviguer vers le résultat sélectionné
        if let Some(&index) = state
            .search_state
            .results
            .get(state.search_state.current_result)
        {
            if index < state.graph_view.len() {
                state.graph_view.select_commit(index);
                // Rafraîchir les fichiers du commit sélectionné
                state.refresh_commit_files();
                // Charger le diff du fichier si disponible
                if !state.graph_view.commit_files.is_empty() {
                    crate::handler::navigation::load_commit_file_diff(state);
                }
            }
        }
    }
    Ok(())
}

fn handle_change_type(state: &mut AppState) -> Result<()> {
    state.search_state.cycle_search_type();
    Ok(())
}

fn handle_execute(state: &mut AppState) -> Result<()> {
    if state.search_state.query.is_empty() {
        return Ok(());
    }

    let query = state.search_state.query.clone();
    let search_type = state.search_state.search_type;

    // Utiliser filter_commits de git::search avec l'API unifiée
    let results =
        crate::git::search::filter_commits(&state.graph_view.rows.items, &query, search_type);

    state.search_state.results = results;
    state.search_state.current_result = 0;

    if !state.search_state.results.is_empty() {
        // Naviguer directement vers le premier résultat
        if let Some(&index) = state.search_state.results.first() {
            if index < state.graph_view.len() {
                state.graph_view.select_commit(index);
                // Rafraîchir les fichiers du commit sélectionné
                state.refresh_commit_files();
                // Charger le diff du fichier si disponible
                if !state.graph_view.commit_files.is_empty() {
                    crate::handler::navigation::load_commit_file_diff(state);
                }
            }
        }
        state.set_flash_message(flash_success(format!(
            "{} résultats trouvés",
            state.search_state.results.len()
        )));
    } else {
        state.set_flash_message("Aucun résultat".to_string());
    }

    Ok(())
}
