//! Disposition des panneaux de la vue branches.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
pub struct BranchesLayout {
    pub status_bar: Rect,
    pub nav_bar: Rect,
    pub tabs: Rect,
    pub list_panel: Rect,
    pub detail_panel: Rect,
    pub help_bar: Rect,
}

/// Construit le layout de la vue branches.
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────┐
/// │  Status Bar (1 ligne)                                        │
/// ├──────────────────────────────────────────────────────────────┤
/// │  [Branches]  [Worktrees]  [Stashes]     ← onglets           │
/// ├────────────────────────────┬─────────────────────────────────┤
/// │                            │                                 │
/// │  Liste (40%)               │  Détail (60%)                   │
/// │                            │                                 │
/// ├────────────────────────────┴─────────────────────────────────┤
/// │  Help bar (2 lignes)                                         │
/// └──────────────────────────────────────────────────────────────┘
/// ```
pub fn build_branches_layout(area: Rect) -> BranchesLayout {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Length(2), // Navigation globale + bordure
            Constraint::Length(1), // Onglets
            Constraint::Min(0),    // Contenu
            Constraint::Length(2), // Help bar
        ])
        .split(area);

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[3]);

    BranchesLayout {
        status_bar: outer[0],
        nav_bar: outer[1],
        tabs: outer[2],
        list_panel: content[0],
        detail_panel: content[1],
        help_bar: outer[4],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branches_layout_reserves_both_navigation_levels() {
        let layout = build_branches_layout(Rect::new(0, 0, 80, 24));

        assert_eq!(layout.nav_bar.height, 2);
        assert_eq!(layout.tabs.height, 1);
        assert_eq!(layout.help_bar.height, 2);
    }
}
