//! Point d'entrée du rendu UI : dispatche vers les vues selon le `ViewMode`.

pub mod blame_view;
pub mod branch_panel;
pub mod branches_layout;
pub mod branches_view;
pub mod common;
pub mod confirm_dialog;
pub mod conflicts_view;
pub mod detail_view;
pub mod diff_view;
pub mod files_view;
pub mod filter_popup;
pub mod graph_legend;
pub mod graph_view;
pub mod help_bar;
pub mod help_overlay;
pub mod input;
pub mod layout;
pub mod loading;
pub mod merge_picker;
pub mod nav_bar;
pub mod reset_picker;
pub mod search_bar;
pub mod staging_layout;
pub mod staging_view;
pub mod status_bar;
pub mod theme;

#[cfg(test)]
mod tests;

use crate::state::{AppState, FocusPanel, ViewMode};
use ratatui::Frame;

/// Point d'entrée du rendu : dessine tous les panneaux.
pub fn render(frame: &mut Frame, state: &mut AppState) {
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
                state.is_merging,
            );
        }
        ViewMode::Help => {
            // Rendre la vue sous-jacente d'abord
            match state.previous_view_mode {
                Some(ViewMode::Staging) => {
                    staging_view::render(
                        frame,
                        &state.staging_state,
                        &state.current_branch,
                        &state.repo_path,
                        state.current_flash_message(),
                        state.is_merging,
                    );
                }
                Some(ViewMode::Branches) => {
                    branches_view::render(
                        frame,
                        &state.branches_view_state,
                        &state.current_branch,
                        &state.repo_path,
                        state.current_flash_message(),
                    );
                }
                Some(ViewMode::Conflicts) | _ if state.conflicts_state.is_some() => {
                    // Extraire les valeurs nécessaires avant l'emprunt mutable
                    let flash_msg = state.current_flash_message().map(|s| s.to_string());
                    let current_branch = state.current_branch.clone();
                    let repo_path = state.repo_path.clone();

                    if let Some(ref mut conflicts_state) = state.conflicts_state {
                        conflicts_view::render(
                            frame,
                            conflicts_state,
                            &current_branch,
                            &repo_path,
                            flash_msg.as_deref(),
                        );
                    }
                    conflicts_view::render_help_overlay(frame, frame.area());
                    return; // L'overlay de conflits est spécifique
                }
                _ => {
                    render_graph_view(frame, state);
                }
            }
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
        ViewMode::Conflicts => {
            // Extraire les valeurs nécessaires avant l'emprunt mutable
            let flash_msg = state.current_flash_message().map(|s| s.to_string());
            let current_branch = state.current_branch.clone();
            let repo_path = state.repo_path.clone();

            if let Some(ref mut conflicts_state) = state.conflicts_state {
                conflicts_view::render(
                    frame,
                    conflicts_state,
                    &current_branch,
                    &repo_path,
                    flash_msg.as_deref(),
                );
            }
        }
    }

    // Rendre le merge picker si actif
    if let Some(ref picker) = state.merge_picker {
        if picker.is_active {
            merge_picker::render(frame, picker, &state.current_branch, frame.area());
        }
    }

    // Rendre le reset picker si actif
    if let Some(ref picker) = state.reset_picker {
        if picker.is_active {
            reset_picker::render(frame, picker, &state.current_branch, frame.area());
        }
    }

    // Rendre le dialogue de confirmation si actif
    if let Some(ref action) = state.pending_confirmation {
        confirm_dialog::render(frame, action, frame.area());
    }
}

/// Rend la vue Graph (vue principale).
fn render_graph_view(frame: &mut Frame, state: &mut AppState) {
    // Utiliser le layout avec support du mode diff plein écran
    let layout = layout::build_layout_with_diff_mode(
        frame.area(),
        state.search_state.is_active,
        state.diff_fullscreen,
    );

    // Rendu de la status bar en haut.
    status_bar::render(
        frame,
        &state.current_branch,
        &state.repo_path,
        &state.status_entries,
        state.current_flash_message(),
        &state.graph_filter,
        state.is_merging,
        layout.status_bar,
    );

    // Rendu de la barre de navigation.
    let unresolved_count = state
        .conflicts_state
        .as_ref()
        .map(|cs| {
            cs.all_files
                .iter()
                .filter(|f| !f.is_resolved && f.has_conflicts)
                .count()
        })
        .unwrap_or(0);
    nav_bar::render(frame, state.view_mode, layout.nav_bar, unresolved_count);

    // Rendu du graphe (masqué en mode diff plein écran).
    if !state.diff_fullscreen {
        let is_graph_focused = state.focus == FocusPanel::Graph;
        // Utiliser le total de commits (si filtres actifs, c'est le dernier total connu)
        let total_commits = state.graph_view.rows.len().max(state.graph.len());
        graph_view::render(
            frame,
            &state.graph,
            &state.current_branch,
            state.graph_filter.is_active(),
            state.selected_index,
            total_commits,
            layout.graph,
            &mut state.graph_state,
            is_graph_focused,
        );
    }

    // Obtenir le hash du commit sélectionné pour le titre.
    let selected_hash = state.graph.get(state.selected_index).map(|row| {
        let hash = row.node.oid.to_string();
        hash[..7].to_string()
    });

    // Rendu du panneau de fichiers (masqué en mode diff plein écran).
    if !state.diff_fullscreen {
        let is_files_focused = state.focus == FocusPanel::BottomLeft;
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
    }

    // Rendu du panneau bas-droit (contextuel selon le focus).
    let is_detail_focused = state.focus == FocusPanel::BottomRight;

    // Si mode diff plein écran est actif, afficher le diff sur toute la zone
    if let Some(diff_area) = layout.diff_fullscreen {
        // En mode plein écran, le diff est toujours visible.
        let total_lines = diff_view::render(
            frame,
            state.selected_file_diff.as_ref(),
            state.diff_scroll_offset,
            state.diff_horizontal_offset,
            diff_area,
            state.focus == FocusPanel::BottomRight,
            state.diff_view_mode,
            true,
        );
        state.diff_total_lines = total_lines;
    } else {
        // Mode normal
        match state.focus {
            FocusPanel::Graph | FocusPanel::BottomRight => {
                // Afficher les métadonnées du commit.
                detail_view::render(
                    frame,
                    &state.graph,
                    state.selected_index,
                    layout.bottom_right,
                    is_detail_focused,
                );
            }
            FocusPanel::BottomLeft => {
                // Afficher le diff du fichier sélectionné.
                let total_lines = diff_view::render(
                    frame,
                    state.selected_file_diff.as_ref(),
                    state.diff_scroll_offset,
                    state.diff_horizontal_offset,
                    layout.bottom_right,
                    false,
                    state.diff_view_mode,
                    false,
                );
                state.diff_total_lines = total_lines;
            }
        }
    }

    // Rendu de la barre d'aide.
    help_bar::render(
        frame,
        state.selected_index,
        state.graph.len(),
        state.bottom_left_mode.clone(),
        state.graph_filter.is_active(),
        state.is_merging,
        layout.help_bar,
    );

    // Rendu de la barre de recherche (si active).
    if let Some(search_area) = layout.search_bar {
        search_bar::render(frame, &state.search_state, search_area);
    }

    // Panneau de branches (si actif).
    if state.show_branch_panel {
        branch_panel::render(frame, &state.branches, state.branch_selected, frame.area());
    }

    // Popup de filtre (si ouvert).
    if state.filter_popup.is_open {
        filter_popup::render(
            frame,
            &state.filter_popup,
            &state.graph_filter,
            frame.area(),
        );
    }
}
