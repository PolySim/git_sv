//! Tests snapshot pour les composants UI.

mod snapshots;

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

/// Helper pour capturer le rendu d'un composant.
pub fn render_to_string<F>(width: u16, height: u16, render_fn: F) -> String
where
    F: FnOnce(&mut ratatui::Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            render_fn(frame);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    buffer_to_string(buffer)
}

fn buffer_to_string(buffer: &Buffer) -> String {
    let mut output = String::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        output.push_str(line.trim_end());
        output.push('\n');
    }
    output
}
