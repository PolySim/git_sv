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

#[test]
#[cfg(feature = "image-preview")]
fn test_leaving_diff_views_releases_preview_source() {
    use crate::git::diff::{ImageFormat, ImagePreview};
    use crate::state::ViewMode;
    use std::sync::Arc;

    let mut harness = crate::test_utils::ui_driver::UiTestHarness::new();
    for view in [ViewMode::Branches, ViewMode::Blame, ViewMode::Conflicts] {
        let preview = ImagePreview {
            bytes: Arc::from(b"image source".as_slice()),
            format: ImageFormat::Raster,
        };
        let source = Arc::downgrade(&preview.bytes);
        render_to_string(80, 24, |frame| {
            let _ = harness
                .state
                .image_preview
                .render(frame, frame.area(), &preview);
        });
        drop(preview);
        assert!(source.upgrade().is_some());
        harness.state.view_mode = view;
        render_to_string(80, 24, |frame| crate::ui::render(frame, &mut harness.state));
        assert!(source.upgrade().is_none());
    }
}
