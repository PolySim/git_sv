//! Handler pour les actions de filtrage du graph.

use crate::error::Result;
use crate::state::AppState;
use crate::state::action::FilterAction;
use super::traits::{ActionHandler, HandlerContext};

/// Handler pour les opérations de filtrage.
pub struct FilterHandler;

impl ActionHandler for FilterHandler {
    type Action = FilterAction;

    fn can_handle(&self, state: &AppState, _action: &FilterAction) -> bool {
        // Le filtre ne peut être utilisé qu'en mode Graph
        state.view_mode == crate::state::ViewMode::Graph
    }

    fn handle(&mut self, ctx: &mut HandlerContext, action: FilterAction) -> Result<()> {
        match action {
            FilterAction::Open => handle_open(ctx.state),
            FilterAction::Close => handle_close(ctx.state),
            FilterAction::NextField => handle_next_field(ctx.state),
            FilterAction::PreviousField => handle_previous_field(ctx.state),
            FilterAction::InsertChar(c) => handle_insert_char(ctx.state, c),
            FilterAction::DeleteChar => handle_delete_char(ctx.state),
            FilterAction::Apply => handle_apply(ctx.state),
            FilterAction::Clear => handle_clear(ctx.state),
        }
    }
}

fn handle_open(state: &mut AppState) -> Result<()> {
    state.filter_popup.open(&state.graph_filter);
    Ok(())
}

fn handle_close(state: &mut AppState) -> Result<()> {
    state.filter_popup.close();
    Ok(())
}

fn handle_next_field(state: &mut AppState) -> Result<()> {
    state.filter_popup.next_field();
    Ok(())
}

fn handle_previous_field(state: &mut AppState) -> Result<()> {
    state.filter_popup.previous_field();
    Ok(())
}

fn handle_insert_char(state: &mut AppState, c: char) -> Result<()> {
    state.filter_popup.current_input_mut().push(c);
    Ok(())
}

fn handle_delete_char(state: &mut AppState) -> Result<()> {
    let input = state.filter_popup.current_input_mut();
    if !input.is_empty() {
        input.pop();
    }
    Ok(())
}

fn handle_apply(state: &mut AppState) -> Result<()> {
    // Appliquer les valeurs du popup au filtre
    state.filter_popup.apply_to_filter(&mut state.graph_filter);

    // Fermer le popup
    state.filter_popup.close();

    // Rafraîchir le graph avec les nouveaux filtres
    state.dirty = true;

    // Afficher un message si des filtres sont actifs
    if state.graph_filter.is_active() {
        let mut parts = Vec::new();
        if state.graph_filter.author.is_some() {
            parts.push("auteur");
        }
        if state.graph_filter.date_from.is_some() || state.graph_filter.date_to.is_some() {
            parts.push("date");
        }
        if state.graph_filter.path.is_some() {
            parts.push("chemin");
        }
        if state.graph_filter.message.is_some() {
            parts.push("message");
        }
        state.set_flash_message(format!("Filtres actifs: {}", parts.join(", ")));
    } else {
        state.set_flash_message("Filtres effacés".to_string());
    }

    Ok(())
}

fn handle_clear(state: &mut AppState) -> Result<()> {
    // Effacer tous les filtres
    state.graph_filter.clear();

    // Vider aussi les inputs du popup
    state.filter_popup.author_input.clear();
    state.filter_popup.date_from_input.clear();
    state.filter_popup.date_to_input.clear();
    state.filter_popup.path_input.clear();
    state.filter_popup.message_input.clear();

    // Fermer le popup
    state.filter_popup.close();

    // Rafraîchir le graph sans filtres
    state.dirty = true;

    state.set_flash_message("Filtres effacés".to_string());

    Ok(())
}
