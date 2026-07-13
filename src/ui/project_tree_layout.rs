//! Disposition des panneaux de la vue arborescence.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct ProjectTreeLayout {
    pub status_bar: Rect,
    pub nav_bar: Rect,
    pub search_bar: Option<Rect>,
    pub tree_panel: Rect,
    pub history_panel: Rect,
    pub changed_files_panel: Rect,
    pub diff_panel: Rect,
    pub help_bar: Rect,
}

pub fn build_project_tree_layout(area: Rect, search_active: bool) -> ProjectTreeLayout {
    let search_height = u16::from(search_active) * 3;
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(search_height),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(outer[3]);
    let details = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(content[1]);
    let commit_details = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(details[1]);

    ProjectTreeLayout {
        status_bar: outer[0],
        nav_bar: outer[1],
        search_bar: search_active.then_some(outer[2]),
        tree_panel: content[0],
        history_panel: details[0],
        changed_files_panel: commit_details[0],
        diff_panel: commit_details[1],
        help_bar: outer[4],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_reserves_navigation_and_help() {
        let layout = build_project_tree_layout(Rect::new(0, 0, 100, 30), false);

        assert_eq!(layout.status_bar.height, 1);
        assert_eq!(layout.nav_bar.height, 2);
        assert_eq!(layout.help_bar.height, 2);
        assert!(
            layout.tree_panel.width < layout.history_panel.width + layout.changed_files_panel.width
        );
        assert!(layout.diff_panel.width > layout.changed_files_panel.width);
        assert!(layout.search_bar.is_none());
    }

    #[test]
    fn layout_reserves_search_when_active() {
        let layout = build_project_tree_layout(Rect::new(0, 0, 100, 30), true);

        assert_eq!(layout.search_bar.expect("barre de recherche").height, 3);
    }
}
