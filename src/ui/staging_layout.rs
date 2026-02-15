use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Structure représentant les zones de la vue staging.
pub struct StagingLayout {
    pub status_bar: Rect,
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
            Constraint::Min(0),    // Contenu principal
            Constraint::Length(3), // Zone message commit
            Constraint::Length(2), // Help bar
        ])
        .split(area);

    // Split horizontal du contenu : listes (40%) + diff (60%)
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[1]);

    // Split vertical de la partie gauche : unstaged (50%) + staged (50%)
    let lists = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(content[0]);

    StagingLayout {
        status_bar: outer[0],
        unstaged_panel: lists[0],
        staged_panel: lists[1],
        diff_panel: content[1],
        commit_message: outer[2],
        help_bar: outer[3],
    }
}
