//! Disposition des panneaux de la vue staging.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
pub struct StagingLayout {
    pub status_bar: Rect,
    pub nav_bar: Rect,
    pub unstaged_panel: Rect,
    pub staged_panel: Rect,
    pub diff_panel: Rect,
    pub commit_message: Rect,
    pub help_bar: Rect,
}

/// Construit le layout de la vue staging.
///
/// ```text
/// ┌──────────────────────────────────────────────────────────┐
/// │  Status Bar (1 ligne)                                    │
/// ├────────────────────────────┬─────────────────────────────┤
/// │  Unstaged (50%)            │                             │
/// │  ┌────────────────────────┐│    Diff du fichier          │
/// │  │ ...                    ││    sélectionné              │
/// │  └────────────────────────┘│                             │
/// │  Staged (50%)              │                             │
/// │  ┌────────────────────────┐│                             │
/// │  │ ...                    ││                             │
/// │  └────────────────────────┘│                             │
/// ├────────────────────────────┴─────────────────────────────┤
/// │  Message de commit                                       │
/// ├──────────────────────────────────────────────────────────┤
/// │  Help bar                                                │
/// └──────────────────────────────────────────────────────────┘
/// ```
pub fn build_staging_layout(area: Rect) -> StagingLayout {
    // Split vertical : status_bar + contenu + message + help_bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Length(2), // Navigation globale + bordure
            Constraint::Min(0),    // Contenu principal
            Constraint::Length(3), // Zone message commit
            Constraint::Length(2), // Help bar
        ])
        .split(area);

    // Split horizontal du contenu : listes (40%) + diff (60%)
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[2]);

    // Split vertical de la partie gauche : unstaged (50%) + staged (50%)
    let lists = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(content[0]);

    StagingLayout {
        status_bar: outer[0],
        nav_bar: outer[1],
        unstaged_panel: lists[0],
        staged_panel: lists[1],
        diff_panel: content[1],
        commit_message: outer[3],
        help_bar: outer[4],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_staging_layout_reserves_global_navigation() {
        let layout = build_staging_layout(Rect::new(0, 0, 80, 24));

        assert_eq!(layout.nav_bar.height, 2);
        assert_eq!(layout.help_bar.height, 2);
        assert!(layout.diff_panel.height >= 3);
    }
}
