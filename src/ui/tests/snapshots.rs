//! Snapshots de rendu pour quelques composants UI critiques.

use insta::assert_snapshot;
use ratatui::layout::Rect;

use super::render_to_string;
use crate::state::{BottomLeftMode, GraphFilter, ViewMode};

#[test]
fn test_snapshot_status_bar_basic() {
    let output = render_to_string(80, 1, |frame| {
        crate::ui::status_bar::render(
            frame,
            crate::ui::status_bar::StatusBarRenderContext {
                current_branch: Some("main"),
                status_entries: &[],
                flash_message: None,
                filter: &GraphFilter::new(),
                is_merging: false,
                area: Rect::new(0, 0, 80, 1),
            },
        );
    });

    assert_snapshot!("status_bar_basic", output);
}

#[test]
fn test_snapshot_help_bar_graph_view() {
    let output = render_to_string(100, 2, |frame| {
        crate::ui::help_bar::render(
            frame,
            crate::ui::help_bar::HelpBarRenderContext {
                selected_index: 0,
                total_commits: 42,
                bottom_left_mode: BottomLeftMode::Files,
                filter_active: false,
                is_merging: false,
                area: Rect::new(0, 0, 100, 2),
            },
        );
    });

    assert_snapshot!("help_bar_graph_view", output);
}

#[test]
fn test_snapshot_nav_bar_graph_selected() {
    let output = render_to_string(80, 2, |frame| {
        crate::ui::nav_bar::render(
            frame,
            crate::ui::nav_bar::NavBarRenderContext {
                current_view: ViewMode::Graph,
                area: Rect::new(0, 0, 80, 2),
                unresolved_conflicts: 0,
            },
        );
    });

    assert_snapshot!("nav_bar_graph_selected", output);
}
