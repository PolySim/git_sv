pub mod detail_view;
pub mod files_view;
pub mod graph_view;
pub mod help_bar;
pub mod help_overlay;
pub mod input;
pub mod layout;

use crate::app::ViewMode;
use ratatui::widgets::ListState;
use ratatui::Frame;

/// Point d'entrée du rendu : dessine tous les panneaux.
pub fn render(
    frame: &mut Frame,
    graph: &[crate::git::graph::GraphRow],
    current_branch: &Option<String>,
    commit_files: &[crate::git::diff::DiffFile],
    status_entries: &[crate::git::repo::StatusEntry],
    selected_index: usize,
    bottom_left_mode: crate::app::BottomLeftMode,
    graph_state: &mut ListState,
    view_mode: crate::app::ViewMode,
) {
    let layout = layout::build_layout(frame.area());

    // Rendu du graphe.
    graph_view::render(
        frame,
        graph,
        current_branch,
        selected_index,
        layout.graph,
        graph_state,
    );

    // Obtenir le hash du commit sélectionné pour le titre.
    let selected_hash = graph.get(selected_index).map(|row| {
        let hash = row.node.oid.to_string();
        hash[..7].to_string()
    });

    // Rendu du panneau de fichiers.
    files_view::render(
        frame,
        commit_files,
        status_entries,
        selected_hash,
        bottom_left_mode.clone(),
        layout.bottom_left,
    );

    // Rendu du panneau de détail.
    detail_view::render(frame, graph, selected_index, layout.bottom_right);

    // Rendu de la barre d'aide.
    help_bar::render(
        frame,
        selected_index,
        graph.len(),
        bottom_left_mode,
        layout.help_bar,
    );

    // Overlay d'aide (si actif).
    if view_mode == ViewMode::Help {
        help_overlay::render(frame, frame.area());
    }
}
