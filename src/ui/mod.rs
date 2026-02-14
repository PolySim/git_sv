pub mod detail_view;
pub mod files_view;
pub mod graph_view;
pub mod input;
pub mod layout;

use crate::app::{BottomLeftMode, ViewMode};
use crate::git::diff::DiffFile;
use crate::git::graph::GraphRow;
use crate::git::repo::StatusEntry;
use ratatui::widgets::ListState;
use ratatui::Frame;

/// Point d'entrée du rendu : dessine tous les panneaux.
pub fn render(
    frame: &mut Frame,
    graph: &[GraphRow],
    current_branch: &Option<String>,
    commit_files: &[DiffFile],
    status_entries: &[StatusEntry],
    selected_index: usize,
    view_mode: ViewMode,
    bottom_left_mode: BottomLeftMode,
    graph_state: &mut ListState,
) {
    let chunks = layout::build_layout(frame.area());

    graph_view::render(
        frame,
        graph,
        current_branch,
        selected_index,
        chunks[0],
        graph_state,
    );

    // Obtenir le hash du commit sélectionné pour le titre.
    let selected_hash = graph.get(selected_index).map(|row| {
        let hash = row.node.oid.to_string();
        hash[..7].to_string()
    });

    files_view::render(
        frame,
        commit_files,
        status_entries,
        selected_hash,
        bottom_left_mode,
        chunks[1],
    );
    detail_view::render(frame, graph, selected_index, chunks[2]);
}
