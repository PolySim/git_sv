use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Structure contenant toutes les zones de layout.
pub struct LayoutChunks {
    /// Zone de la status bar (1 ligne en haut).
    pub status_bar: Rect,
    /// Zone de la navigation bar (1 ligne sous la status bar).
    pub nav_bar: Rect,
    /// Zone du graphe (partie supérieure).
    pub graph: Rect,
    /// Zone du panneau bas-gauche (fichiers/status).
    pub bottom_left: Rect,
    /// Zone du panneau bas-droit (détail).
    pub bottom_right: Rect,
    /// Zone de la barre de recherche (3 lignes en bas, optionnelle).
    pub search_bar: Option<Rect>,
    /// Zone de la barre d'aide (1 ligne en bas).
    pub help_bar: Rect,
}

/// Construit le layout principal de l'application.
///
/// Disposition sans recherche :
/// ┌───────────────────────────┐
/// │    Status Bar (1 ligne)   │
/// ├───────────────────────────┤
/// │    Navigation Bar (1)     │
/// ├───────────────────────────┤
/// │       Graph (60%)         │
/// ├──────────────┬────────────┤
/// │ Files (50%)  │Detail (50%)│
/// ├──────────────┴────────────┤
/// │       Help Bar (1 ligne)  │
/// └───────────────────────────┘
///
/// Disposition avec recherche :
/// ┌───────────────────────────┐
/// │    Status Bar (1 ligne)   │
/// ├───────────────────────────┤
/// │    Navigation Bar (1)     │
/// ├───────────────────────────┤
/// │       Graph (60%)         │
/// ├──────────────┬────────────┤
/// │ Files (50%)  │Detail (50%)│
/// ├──────────────┴────────────┤
/// │      Search Bar (3 lignes)│
/// ├───────────────────────────┤
/// │       Help Bar (1 ligne)  │
/// └───────────────────────────┘
pub fn build_layout(area: Rect, show_search: bool) -> LayoutChunks {
    let search_height = if show_search { 3 } else { 0 };

    // Split vertical : status bar + nav bar + contenu principal + [search bar] + help bar.
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),             // Status bar
            Constraint::Length(1),             // Navigation bar
            Constraint::Min(0),                // Contenu principal
            Constraint::Length(search_height), // Search bar (optionnel)
            Constraint::Length(1),             // Help bar
        ])
        .split(area);

    // Split du contenu principal : graphe (60%) + bas (40%).
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer[2]);

    // Split de la partie basse : gauche (50%) + droite (50%).
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    LayoutChunks {
        status_bar: outer[0],
        nav_bar: outer[1],
        graph: main_chunks[0],
        bottom_left: bottom_chunks[0],
        bottom_right: bottom_chunks[1],
        search_bar: if show_search { Some(outer[3]) } else { None },
        help_bar: outer[4],
    }
}
