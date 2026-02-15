pub mod branch_panel;
pub mod branches_layout;
pub mod branches_view;
pub mod detail_view;
pub mod diff_view;
pub mod files_view;
pub mod graph_view;
pub mod help_bar;
pub mod help_overlay;
pub mod input;
pub mod layout;
pub mod staging_layout;
pub mod staging_view;
pub mod status_bar;

use crate::app::{BranchesViewState, FocusPanel, StagingState, ViewMode};
use crate::git::diff::FileDiff;
use ratatui::widgets::ListState;
use ratatui::Frame;

/// Point d'entrée du rendu : dessine tous les panneaux.
#[allow(clippy::too_many_arguments)]
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
    file_selected_index: usize,
    selected_file_diff: Option<&FileDiff>,
    diff_scroll_offset: usize,
    staging_state: &StagingState,
    branches_view_state: &BranchesViewState,
) {
    // Dispatcher le rendu selon le mode de vue
    match view_mode {
        ViewMode::Graph => {
            render_graph_view(
                frame,
                graph,
                current_branch,
                commit_files,
                status_entries,
                branches,
                selected_index,
                branch_selected,
                bottom_left_mode,
                focus,
                graph_state,
                show_branch_panel,
                repo_path,
                flash_message,
                file_selected_index,
                selected_file_diff,
                diff_scroll_offset,
            );
        }
        ViewMode::Staging => {
            staging_view::render(
                frame,
                staging_state,
                current_branch,
                repo_path,
                flash_message,
            );
        }
        ViewMode::Help => {
            render_graph_view(
                frame,
                graph,
                current_branch,
                commit_files,
                status_entries,
                branches,
                selected_index,
                branch_selected,
                bottom_left_mode,
                focus,
                graph_state,
                show_branch_panel,
                repo_path,
                flash_message,
                file_selected_index,
                selected_file_diff,
                diff_scroll_offset,
            );
            help_overlay::render(frame, frame.area());
        }
        ViewMode::Branches => {
            branches_view::render(
                frame,
                branches_view_state,
                current_branch,
                repo_path,
                flash_message,
            );
        }
    }
}

/// Rend la vue Graph (vue principale).
#[allow(clippy::too_many_arguments)]
fn render_graph_view(
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
    show_branch_panel: bool,
    repo_path: &str,
    flash_message: Option<&str>,
    file_selected_index: usize,
    selected_file_diff: Option<&FileDiff>,
    diff_scroll_offset: usize,
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
        file_selected_index,
    );

    // Rendu du panneau bas-droit (contextuel selon le focus).
    let is_detail_focused = focus == crate::app::FocusPanel::Detail;

    match focus {
        FocusPanel::Graph | FocusPanel::Detail => {
            // Afficher les métadonnées du commit.
            detail_view::render(
                frame,
                graph,
                selected_index,
                layout.bottom_right,
                is_detail_focused,
            );
        }
        FocusPanel::Files => {
            // Afficher le diff du fichier sélectionné.
            diff_view::render(
                frame,
                selected_file_diff,
                diff_scroll_offset,
                layout.bottom_right,
                false,
            );
        }
    }

    // Rendu de la barre d'aide.
    help_bar::render(
        frame,
        selected_index,
        graph.len(),
        bottom_left_mode,
        layout.help_bar,
    );

    // Panneau de branches (si actif).
    if show_branch_panel {
        branch_panel::render(frame, branches, branch_selected, frame.area());
    }
}
