pub mod branch_panel;
pub mod detail_view;
pub mod files_view;
pub mod graph_view;
pub mod help_bar;
pub mod help_overlay;
pub mod input;
pub mod layout;
pub mod status_bar;

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
    branches: &[crate::git::branch::BranchInfo],
    selected_index: usize,
    branch_selected: usize,
    bottom_left_mode: crate::app::BottomLeftMode,
    focus: crate::app::FocusPanel,
    graph_state: &mut ListState,
    view_mode: crate::app::ViewMode,
    show_branch_panel: bool,
    repo_path: &str,
    flash_message: Option<&str>,
) {
    let layout = layout::build_layout(frame.area());

    // Rendu de la status bar en haut.
    status_bar::render(
        frame,
        current_branch,
        repo_path,
        status_entries,
        flash_message,
        layout.status_bar,
    );

    // Rendu du graphe.
    let is_graph_focused = focus == crate::app::FocusPanel::Graph;
    graph_view::render(
        frame,
        graph,
        current_branch,
        selected_index,
        layout.graph,
        graph_state,
        is_graph_focused,
    );

    // Obtenir le hash du commit sélectionné pour le titre.
    let selected_hash = graph.get(selected_index).map(|row| {
        let hash = row.node.oid.to_string();
        hash[..7].to_string()
    });

    // Rendu du panneau de fichiers.
    let is_files_focused = focus == crate::app::FocusPanel::Files;
    files_view::render(
        frame,
        commit_files,
        status_entries,
        selected_hash,
        bottom_left_mode.clone(),
        layout.bottom_left,
        is_files_focused,
    );

    // Rendu du panneau de détail.
    let is_detail_focused = focus == crate::app::FocusPanel::Detail;
    detail_view::render(
        frame,
        graph,
        selected_index,
        layout.bottom_right,
        is_detail_focused,
    );

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

    // Panneau de branches (si actif).
    if show_branch_panel {
        branch_panel::render(frame, branches, branch_selected, frame.area());
    }
}
