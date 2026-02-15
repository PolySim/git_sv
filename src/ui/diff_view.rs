use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::git::diff::{DiffLineType, FileDiff};

/// Rend le diff d'un fichier avec coloration syntaxique.
pub fn render(
    frame: &mut Frame,
    diff: Option<&FileDiff>,
    scroll_offset: usize,
    area: Rect,
    is_focused: bool,
) {
    let content = match diff {
        Some(d) => build_diff_lines(d),
        None => vec![Line::from("Sélectionnez un fichier pour voir le diff")],
    };

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let title = match diff {
        Some(d) => format!(" Diff — {} (+{}/-{}) ", d.path, d.additions, d.deletions),
        None => " Diff ".to_string(),
    };

    let paragraph = Paragraph::new(content)
        .scroll((scroll_offset as u16, 0))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        );

    frame.render_widget(paragraph, area);
}

/// Construit les lignes de diff avec coloration.
fn build_diff_lines(diff: &FileDiff) -> Vec<Line<'static>> {
    diff.lines
        .iter()
        .map(|line| {
            let (prefix, fg_color, bg_color) = match line.line_type {
                DiffLineType::Addition => ("+", Color::Green, Some(Color::Rgb(0, 40, 0))),
                DiffLineType::Deletion => ("-", Color::Red, Some(Color::Rgb(40, 0, 0))),
                DiffLineType::Context => (" ", Color::White, None),
                DiffLineType::HunkHeader => ("", Color::Cyan, None),
            };

            let mut spans = Vec::new();

            if line.line_type == DiffLineType::HunkHeader {
                // Header de hunk : pas de numéros de ligne, juste le contenu en cyan.
                spans.push(Span::styled(
                    line.content.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ));
            } else {
                // Numéros de lignes.
                let old_no = line
                    .old_lineno
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());
                let new_no = line
                    .new_lineno
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());
                spans.push(Span::styled(
                    format!("{} {} ", old_no, new_no),
                    Style::default().fg(Color::DarkGray),
                ));

                // Préfixe et contenu avec coloration.
                let style = Style::default().fg(fg_color);
                let style = if let Some(bg) = bg_color {
                    style.bg(bg)
                } else {
                    style
                };
                spans.push(Span::styled(format!("{}{}", prefix, line.content), style));
            }

            Line::from(spans)
        })
        .collect()
}
