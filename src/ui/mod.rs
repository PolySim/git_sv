//! Point d'entrée du rendu UI : dispatche vers les vues selon le `ViewMode`.

pub mod blame_view;
pub mod branches_layout;
pub mod branches_view;
pub mod common;
pub mod confirm_dialog;
pub mod conflicts_view;
pub mod detail_view;
pub mod diff_view;
pub mod files_view;
pub mod filter_popup;
pub mod graph_view;
pub mod help_bar;
pub mod help_overlay;
pub mod hit_test;
pub mod image_preview;
pub mod input;
pub mod keybindings;
pub mod layout;
pub mod loading;
pub mod merge_picker;
pub mod nav_bar;
pub mod reset_picker;
pub mod search_bar;
pub mod staging_layout;
pub mod staging_view;
pub mod status_bar;
pub(crate) mod text_edit;
pub mod theme;

#[cfg(test)]
mod tests;

use crate::git::diff::DiffViewMode;
use crate::state::{AppState, BottomLeftMode, FocusPanel, ViewMode};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Widget},
    Frame,
};

/// Snapshot immuable de l'état pour le rendu de la vue graphe.
struct GraphViewSnapshot {
    is_diff_fullscreen: bool,
    is_search_active: bool,
    graph_len: usize,
    selected_index: usize,
    current_branch: Option<String>,
    filter_active: bool,
    loaded_count: usize,
    total_commits: Option<usize>,
    can_load_more: bool,
    is_loading_more: bool,
    focus: FocusPanel,
    bottom_left_mode: BottomLeftMode,
    diff_scroll_offset: usize,
    diff_horizontal_offset: usize,
    diff_view_mode: DiffViewMode,
    flash_message: Option<String>,
    is_merging: bool,
    unresolved_conflicts: usize,
    selected_hash: Option<String>,
    current_view: ViewMode,
}

impl AppState {
    fn unresolved_conflict_count(&self) -> usize {
        self.conflicts_state
            .as_ref()
            .map(|conflicts| {
                conflicts
                    .all_files
                    .iter()
                    .filter(|file| !file.is_resolved && file.has_conflicts)
                    .count()
            })
            .unwrap_or(0)
    }

    /// Crée un snapshot immuable pour le rendu de la vue graphe.
    fn graph_view_snapshot(&self) -> GraphViewSnapshot {
        let unresolved_conflicts = self.unresolved_conflict_count();

        let selected_hash = self.graph_view.selected_commit().map(|node| {
            let hash = node.oid.to_string();
            hash[..7].to_string()
        });

        GraphViewSnapshot {
            is_diff_fullscreen: self.graph_view.diff_fullscreen,
            is_search_active: self.search_state.is_active,
            graph_len: self.graph_view.len(),
            selected_index: self.graph_view.selected_index(),
            current_branch: self.current_branch.clone(),
            filter_active: self.filters.graph_filter.is_active(),
            loaded_count: self.graph_view.loaded_count,
            total_commits: self.graph_view.total_commits,
            can_load_more: self.graph_view.can_load_more,
            is_loading_more: self.graph_view.is_loading_more,
            focus: self.focus,
            bottom_left_mode: self.bottom_left_mode,
            diff_scroll_offset: self.graph_view.diff_scroll_offset,
            diff_horizontal_offset: self.graph_view.diff_horizontal_offset,
            diff_view_mode: self.graph_view.diff_view_mode,
            flash_message: self
                .current_flash_message()
                .map(|message| message.to_string()),
            is_merging: self.ui.is_merging,
            unresolved_conflicts,
            selected_hash,
            current_view: self.view_mode,
        }
    }
}

fn render_conflicts(frame: &mut Frame, state: &mut AppState) {
    let flash_msg = state.current_flash_message().map(|s| s.to_string());
    let current_branch = state.current_branch.clone();
    let repo_path = state.repo_path.clone();

    if let Some(ref mut conflicts_state) = state.conflicts_state {
        conflicts_view::render(
            frame,
            conflicts_view::ConflictsRenderContext {
                state: conflicts_state,
                current_branch: current_branch.as_deref(),
                repo_path: &repo_path,
                flash_message: flash_msg.as_deref(),
            },
        );
    }
}

fn render_conflicts_help_overlay(frame: &mut Frame) {
    conflicts_view::render_help_overlay(
        frame,
        conflicts_view::ConflictsHelpOverlayRenderContext { area: frame.area() },
    );
}

fn render_global_overlays(frame: &mut Frame, state: &AppState) {
    if let Some(ref picker) = state.merge_picker {
        if picker.is_active {
            merge_picker::render(
                frame,
                merge_picker::MergePickerRenderContext {
                    state: picker,
                    current_branch: state.current_branch.as_deref(),
                    area: frame.area(),
                },
            );
        }
    }

    if let Some(ref picker) = state.reset_picker {
        if picker.is_active {
            reset_picker::render(
                frame,
                reset_picker::ResetPickerRenderContext {
                    state: picker,
                    area: frame.area(),
                },
            );
        }
    }

    if let Some(ref action) = state.ui.pending_confirmation {
        confirm_dialog::render(
            frame,
            confirm_dialog::ConfirmDialogRenderContext {
                action,
                area: frame.area(),
            },
        );
    }
}

fn render_graph_overlays(frame: &mut Frame, state: &AppState) {
    if state.filters.filter_popup.is_open {
        filter_popup::render(
            frame,
            filter_popup::FilterPopupRenderContext {
                popup_state: &state.filters.filter_popup,
                current_filter: &state.filters.graph_filter,
                area: frame.area(),
            },
        );
    }
}

fn render_graph_status_bar(
    frame: &mut Frame,
    snap: &GraphViewSnapshot,
    state: &AppState,
    area: Rect,
) {
    status_bar::render(
        frame,
        status_bar::StatusBarRenderContext {
            current_branch: snap.current_branch.as_deref(),
            status_entries: &state.status_entries,
            flash_message: snap.flash_message.as_deref(),
            filter: &state.filters.graph_filter,
            is_merging: snap.is_merging,
            area,
        },
    );
}

fn render_graph_nav_bar(frame: &mut Frame, snap: &GraphViewSnapshot, area: Rect) {
    nav_bar::render(
        frame,
        nav_bar::NavBarRenderContext {
            current_view: snap.current_view,
            area,
            unresolved_conflicts: snap.unresolved_conflicts,
        },
    );
}

fn render_graph_panel(
    frame: &mut Frame,
    snap: &GraphViewSnapshot,
    state: &mut AppState,
    area: Rect,
) {
    let list_state = &mut state.graph_view.list_state;
    let rows = state.graph_view.rows.items();

    graph_view::render(
        frame,
        graph_view::GraphRenderContext {
            graph: rows,
            current_branch: snap.current_branch.as_deref(),
            filter_active: snap.filter_active,
            selected_index: snap.selected_index,
            loaded_count: snap.loaded_count,
            total_commits: snap.total_commits,
            can_load_more: snap.can_load_more,
            is_loading_more: snap.is_loading_more,
            area,
            state: list_state,
            is_focused: snap.focus == FocusPanel::Graph,
        },
    );
}

fn render_graph_files_panel(
    frame: &mut Frame,
    snap: &GraphViewSnapshot,
    state: &AppState,
    area: Rect,
) {
    files_view::render(
        frame,
        files_view::FilesRenderContext {
            commit_files: &state.graph_view.commit_files,
            status_entries: &state.status_entries,
            selected_commit_hash: snap.selected_hash.as_deref(),
            mode: snap.bottom_left_mode,
            area,
            is_focused: snap.focus == FocusPanel::BottomLeft,
            file_selected_index: state.graph_view.file_selected_index,
        },
    );
}

fn render_graph_diff_panel(
    frame: &mut Frame,
    snap: &GraphViewSnapshot,
    state: &mut AppState,
    default_area: Rect,
    diff_fullscreen_area: Option<Rect>,
) {
    if let Some(area) = diff_fullscreen_area {
        let total_lines = diff_view::render(
            frame,
            diff_view::DiffRenderContext {
                diff: state.graph_view.selected_file_diff.as_ref(),
                scroll_offset: snap.diff_scroll_offset,
                horizontal_offset: snap.diff_horizontal_offset,
                area,
                is_focused: snap.focus == FocusPanel::BottomRight,
                view_mode: snap.diff_view_mode,
                is_fullscreen: true,
                image_state: &mut state.image_preview,
            },
        );
        state.graph_view.diff_total_lines = total_lines;
        return;
    }

    if matches!(snap.focus, FocusPanel::BottomLeft | FocusPanel::BottomRight) {
        let total_lines = diff_view::render(
            frame,
            diff_view::DiffRenderContext {
                diff: state.graph_view.selected_file_diff.as_ref(),
                scroll_offset: snap.diff_scroll_offset,
                horizontal_offset: snap.diff_horizontal_offset,
                area: default_area,
                is_focused: snap.focus == FocusPanel::BottomRight,
                view_mode: snap.diff_view_mode,
                is_fullscreen: false,
                image_state: &mut state.image_preview,
            },
        );
        state.graph_view.diff_total_lines = total_lines;
        return;
    }

    let rows = state.graph_view.rows.items();
    detail_view::render(
        frame,
        detail_view::DetailRenderContext {
            graph: rows,
            selected_index: snap.selected_index,
            area: default_area,
            is_focused: false,
        },
    );
}

fn render_graph_help_bar(frame: &mut Frame, snap: &GraphViewSnapshot, area: Rect) {
    help_bar::render(
        frame,
        help_bar::HelpBarRenderContext {
            selected_index: snap.selected_index,
            total_commits: snap.graph_len,
            bottom_left_mode: snap.bottom_left_mode,
            filter_active: snap.filter_active,
            is_merging: snap.is_merging,
            area,
        },
    );
}

fn render_graph_search_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    search_bar::render(
        frame,
        search_bar::SearchBarRenderContext {
            search_state: &state.search_state,
            area,
        },
    );
}

/// Point d'entrée du rendu : dessine tous les panneaux.
pub fn render(frame: &mut Frame, state: &mut AppState) {
    state.update_screen_area(frame.area());
    Block::default()
        .style(Style::default().bg(theme::current_theme().background))
        .render(frame.area(), frame.buffer_mut());

    // Dispatcher le rendu selon le mode de vue
    match state.view_mode {
        ViewMode::Graph => {
            render_graph_view(frame, state);
        }
        ViewMode::Staging => {
            let flash_message = state.current_flash_message().map(str::to_string);
            staging_view::render(
                frame,
                staging_view::StagingRenderContext {
                    staging_state: &state.staging_state,
                    current_branch: state.current_branch.as_deref(),
                    repo_path: &state.repo_path,
                    flash_message: flash_message.as_deref(),
                    is_merging: state.ui.is_merging,
                    unresolved_conflicts: state.unresolved_conflict_count(),
                    image_state: &mut state.image_preview,
                },
            );
        }
        ViewMode::Help => {
            // Rendre la vue sous-jacente d'abord
            match state.previous_view_mode {
                Some(ViewMode::Staging) => {
                    let flash_message = state.current_flash_message().map(str::to_string);
                    staging_view::render(
                        frame,
                        staging_view::StagingRenderContext {
                            staging_state: &state.staging_state,
                            current_branch: state.current_branch.as_deref(),
                            repo_path: &state.repo_path,
                            flash_message: flash_message.as_deref(),
                            is_merging: state.ui.is_merging,
                            unresolved_conflicts: state.unresolved_conflict_count(),
                            image_state: &mut state.image_preview,
                        },
                    );
                }
                Some(ViewMode::Branches) => {
                    branches_view::render(
                        frame,
                        branches_view::BranchesRenderContext {
                            state: &state.branches_view_state,
                            current_branch: state.current_branch.as_deref(),
                            repo_path: &state.repo_path,
                            flash_message: state.current_flash_message(),
                            unresolved_conflicts: state.unresolved_conflict_count(),
                        },
                    );
                }
                Some(ViewMode::Conflicts) => {
                    render_conflicts(frame, state);
                    render_conflicts_help_overlay(frame);
                    return; // L'overlay de conflits est spécifique
                }
                _ => {
                    if state.conflicts_state.is_some() {
                        render_conflicts(frame, state);
                        render_conflicts_help_overlay(frame);
                        return;
                    }

                    render_graph_view(frame, state);
                }
            }
            help_overlay::render(
                frame,
                help_overlay::HelpOverlayRenderContext {
                    area: frame.area(),
                    active_view: state.previous_view_mode.unwrap_or(ViewMode::Graph),
                },
            );
        }
        ViewMode::Branches => {
            branches_view::render(
                frame,
                branches_view::BranchesRenderContext {
                    state: &state.branches_view_state,
                    current_branch: state.current_branch.as_deref(),
                    repo_path: &state.repo_path,
                    flash_message: state.current_flash_message(),
                    unresolved_conflicts: state.unresolved_conflict_count(),
                },
            );
        }
        ViewMode::Blame => {
            if let Some(ref blame_state) = state.blame_state {
                frame.render_widget(blame_view::BlameView::new(blame_state), frame.area());
            }
        }
        ViewMode::Conflicts => {
            render_conflicts(frame, state);
        }
    }

    render_global_overlays(frame, state);
}

fn render_graph_view(frame: &mut Frame, state: &mut AppState) {
    let snap = state.graph_view_snapshot();
    let layout = layout::build_layout_with_diff_mode(
        frame.area(),
        snap.is_search_active,
        snap.is_diff_fullscreen,
    );

    let graph_visible_height = layout.graph.height.saturating_sub(2) as usize;
    state
        .graph_view
        .rows
        .set_visible_height((graph_visible_height / 2).max(1));

    render_graph_status_bar(frame, &snap, state, layout.status_bar);
    render_graph_nav_bar(frame, &snap, layout.nav_bar);

    if !snap.is_diff_fullscreen {
        render_graph_panel(frame, &snap, state, layout.graph);
        render_graph_files_panel(frame, &snap, state, layout.bottom_left);
    }

    render_graph_diff_panel(
        frame,
        &snap,
        state,
        layout.bottom_right,
        layout.diff_fullscreen,
    );
    render_graph_help_bar(frame, &snap, layout.help_bar);

    if let Some(search_area) = layout.search_bar {
        render_graph_search_bar(frame, state, search_area);
    }

    render_graph_overlays(frame, state);
}
