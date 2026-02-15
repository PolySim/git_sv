use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Structure représentant les zones de la vue branches.
pub struct BranchesLayout {
    pub status_bar: Rect,
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
            Constraint::Length(1), // Onglets
            Constraint::Min(0),    // Contenu
            Constraint::Length(2), // Help bar
        ])
        .split(area);

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[2]);

    BranchesLayout {
        status_bar: outer[0],
        tabs: outer[1],
        list_panel: content[0],
        detail_panel: content[1],
        help_bar: outer[3],
    }
}

/// Calcule un rectangle centré dans la zone donnée.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
