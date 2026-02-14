use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Structure contenant toutes les zones de layout.
pub struct LayoutChunks {
    /// Zone du graphe (partie supérieure).
    pub graph: Rect,
    /// Zone du panneau bas-gauche (fichiers/status).
    pub bottom_left: Rect,
    /// Zone du panneau bas-droit (détail).
    pub bottom_right: Rect,
    /// Zone de la barre d'aide (1 ligne en bas).
    pub help_bar: Rect,
}

/// Construit le layout principal de l'application.
///
/// Disposition :
/// ┌───────────────────────────┐
/// │       Graph (60%)         │
/// ├──────────────┬────────────┤
/// │ Status (50%) │Detail (50%)│
/// ├──────────────┴────────────┤
/// │       Help Bar (1 ligne)  │
/// └───────────────────────────┘
pub fn build_layout(area: Rect) -> LayoutChunks {
    // Split vertical : contenu principal + help bar.
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // Split du contenu principal : graphe (60%) + bas (40%).
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer[0]);

    // Split de la partie basse : gauche (50%) + droite (50%).
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    LayoutChunks {
        graph: main_chunks[0],
        bottom_left: bottom_chunks[0],
        bottom_right: bottom_chunks[1],
        help_bar: outer[1],
    }
}
