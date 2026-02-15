pub mod blame_view;
pub mod branch_panel;
pub mod branches_layout;
pub mod branches_view;
pub mod common;
pub mod confirm_dialog;
pub mod detail_view;
pub mod diff_view;
pub mod files_view;
pub mod graph_legend;
pub mod graph_view;
pub mod help_bar;
pub mod help_overlay;
pub mod input;
pub mod layout;
pub mod loading;
pub mod nav_bar;
pub mod staging_layout;
pub mod staging_view;
pub mod status_bar;
pub mod theme;

use crate::state::{AppState, FocusPanel, ViewMode};
use ratatui::Frame;

/// Point d'entrée du rendu : dessine tous les panneaux.
pub fn render(frame: &mut Frame, state: &AppState) {
    // Dispatcher le rendu selon le mode de vue
    match state.view_mode {
        ViewMode::Graph => {
            render_graph_view(frame, state);
        }
        ViewMode::Staging => {
            staging_view::render(
                frame,
                &state.staging_state,
                &state.current_branch,
                &state.repo_path,
                state.current_flash_message(),
            );
        }
        ViewMode::Help => {
            render_graph_view(frame, state);
            help_overlay::render(frame, frame.area());
        }
        ViewMode::Branches => {
            branches_view::render(
                frame,
                &state.branches_view_state,
                &state.current_branch,
                &state.repo_path,
                state.current_flash_message(),
            );
        }
        ViewMode::Blame => {
            if let Some(ref blame_state) = state.blame_state {
                frame.render_widget(blame_view::BlameView::new(blame_state), frame.area());
            }
        }
    }

    // Rendre le dialogue de confirmation si actif
    if let Some(ref action) = state.pending_confirmation {
        confirm_dialog::render(frame, action, frame.area());
    }
}

/// Rend la vue Graph (vue principale).
fn render_graph_view(frame: &mut Frame, state: &AppState) {
    let layout = layout::build_layout(frame.area());

    // Rendu de la status bar en haut.
    status_bar::render(
        frame,
        &state.current_branch,
        &state.repo_path,
        &state.status_entries,
        state.current_flash_message(),
        layout.status_bar,
    );

    // Rendu de la barre de navigation.
    nav_bar::render(frame, state.view_mode, layout.nav_bar);

    // Rendu du graphe.
    let is_graph_focused = state.focus == FocusPanel::Graph;
    graph_view::render(
        frame,
        &state.graph,
        &state.current_branch,
        state.selected_index,
        layout.graph,
        &mut state.graph_state.clone(),
        is_graph_focused,
    );

    // Obtenir le hash du commit sélectionné pour le titre.
    let selected_hash = state.graph.get(state.selected_index).map(|row| {
        let hash = row.node.oid.to_string();
        hash[..7].to_string()
    });

    // Rendu du panneau de fichiers.
    let is_files_focused = state.focus == FocusPanel::Files;
    files_view::render(
        frame,
        &state.commit_files,
        &state.status_entries,
        selected_hash,
        state.bottom_left_mode.clone(),
        layout.bottom_left,
        is_files_focused,
        state.file_selected_index,
    );

    // Rendu du panneau bas-droit (contextuel selon le focus).
    let is_detail_focused = state.focus == FocusPanel::Detail;

    match state.focus {
        FocusPanel::Graph | FocusPanel::Detail => {
            // Afficher les métadonnées du commit.
            detail_view::render(
                frame,
                &state.graph,
                state.selected_index,
                layout.bottom_right,
                is_detail_focused,
            );
        }
        FocusPanel::Files => {
            // Afficher le diff du fichier sélectionné.
            diff_view::render(
                frame,
                state.selected_file_diff.as_ref(),
                state.diff_scroll_offset,
                layout.bottom_right,
                false,
            );
        }
    }

    // Rendu de la barre d'aide.
    help_bar::render(
        frame,
        state.selected_index,
        state.graph.len(),
        state.bottom_left_mode.clone(),
        layout.help_bar,
    );

    // Panneau de branches (si actif).
    if state.show_branch_panel {
        branch_panel::render(frame, &state.branches, state.branch_selected, frame.area());
    }
}
