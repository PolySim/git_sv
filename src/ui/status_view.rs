use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::App;

/// Rend le panneau de status dans la zone donnée.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .status_entries
        .iter()
        .map(|entry| {
            let status_color = match entry.display_status() {
                "Nouveau (staged)" | "Modifié (staged)" | "Supprimé (staged)" => Color::Green,
                "Modifié" | "Supprimé" => Color::Red,
                "Non suivi" => Color::DarkGray,
                _ => Color::White,
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:>18} ", entry.display_status()),
                    Style::default().fg(status_color),
                ),
                Span::raw(&entry.path),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = format!(" Status ({} fichiers) ", app.status_entries.len());

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL),
    );

    frame.render_widget(list, area);
}
