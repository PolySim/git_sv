use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Construit le layout principal de l'application.
///
/// Disposition :
/// ┌───────────────────────────┐
/// │       Graph (60%)         │
/// ├──────────────┬────────────┤
/// │ Status (50%) │Detail (50%)│
/// └──────────────┴────────────┘
pub fn build_layout(area: Rect) -> Vec<Rect> {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    vec![main_chunks[0], bottom_chunks[0], bottom_chunks[1]]
}
