pub mod graph_view;
pub mod status_view;
pub mod detail_view;
pub mod input;
pub mod layout;

use ratatui::Frame;
use crate::app::App;

/// Point d'entrée du rendu : dessine tous les panneaux.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = layout::build_layout(frame.area());

    graph_view::render(frame, app, chunks[0]);
    status_view::render(frame, app, chunks[1]);
    detail_view::render(frame, app, chunks[2]);
}
