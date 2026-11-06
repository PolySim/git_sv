//! Rendu commun des selections dans les champs de texte.

use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};

use crate::state::selection_range;

/// Construit un texte dont la selection est mise en evidence.
pub(crate) fn text_with_selection(
    text: &str,
    cursor: usize,
    selection_anchor: Option<usize>,
    normal_style: Style,
    selection_style: Style,
) -> Text<'static> {
    let selection = selection_range(cursor, selection_anchor);
    let mut lines = vec![Line::default()];

    visit_graphemes(text, |symbol, start, end| {
        if symbol == "\n" {
            lines.push(Line::default());
            return;
        }
        let style = if selection
            .as_ref()
            .is_some_and(|range| range.start < end && start < range.end)
        {
            selection_style
        } else {
            normal_style
        };
        append_styled(
            &mut lines.last_mut().expect("au moins une ligne").spans,
            &crate::utils::terminal_text(symbol),
            style,
        );
    });

    Text::from(lines)
}

/// Construit un champ sur une ligne avec un curseur visuel sans découper les graphèmes.
pub(crate) fn line_with_cursor(
    text: &str,
    cursor: usize,
    anchor: Option<usize>,
    normal_style: Style,
    selection_style: Style,
    cursor_style: Style,
) -> Line<'static> {
    let cursor = cursor.min(text.chars().count());
    let selection = selection_range(cursor, anchor);
    let mut spans = Vec::new();
    visit_graphemes(text, |symbol, start, end| {
        let style = if start <= cursor && cursor < end {
            cursor_style
        } else if selection
            .as_ref()
            .is_some_and(|range| range.start < end && start < range.end)
        {
            selection_style
        } else {
            normal_style
        };
        append_styled(&mut spans, &crate::utils::terminal_text(symbol), style);
    });
    if cursor == text.chars().count() {
        spans.push(Span::styled(" ", cursor_style));
    }
    Line::from(spans)
}

/// Regroupe les caractères voisins pour éviter une allocation et un Span par caractère collé.
fn append_styled(spans: &mut Vec<Span<'static>>, text: &str, style: Style) {
    match spans.last_mut() {
        Some(span) if span.style == style => span.content.to_mut().push_str(text),
        _ => spans.push(Span::styled(text.to_string(), style)),
    }
}

/// Visite les graphèmes en conservant les indices de caractères utilisés par l'éditeur.
fn visit_graphemes(text: &str, mut visit: impl FnMut(&str, usize, usize)) {
    let mut index = 0;
    for section in text.split_inclusive('\n') {
        let span = Span::raw(section.strip_suffix('\n').unwrap_or(section));
        for grapheme in span.styled_graphemes(Style::default()) {
            let end = index + grapheme.symbol.chars().count();
            visit(grapheme.symbol, index, end);
            index = end;
        }
        if section.ends_with('\n') {
            visit("\n", index, index + 1);
            index += 1;
        }
    }
}

/// Garde le curseur et le texte qui l'entoure dans le panneau, même pour un collage très long.
pub(crate) fn input_viewport(
    rendered: Text<'static>,
    source: &str,
    cursor: usize,
    width: u16,
    height: u16,
) -> (Text<'static>, Option<(u16, u16)>) {
    if width == 0 || height == 0 {
        return (Text::default(), None);
    }
    let before: String = source.chars().take(cursor).collect();
    let row = before.bytes().filter(|byte| *byte == b'\n').count();
    let column = Span::raw(crate::utils::terminal_text(
        before.rsplit('\n').next().unwrap_or_default(),
    ))
    .width();
    let top = row.saturating_sub(usize::from(height) - 1);
    let left = column.saturating_sub(usize::from(width) - 1);
    let lines = rendered
        .lines
        .into_iter()
        .skip(top)
        .take(usize::from(height))
        .map(|line| clip_line(line, left, usize::from(width)))
        .collect::<Vec<_>>();
    (
        Text::from(lines),
        Some(((column - left) as u16, (row - top) as u16)),
    )
}

/// Découpe une ligne en cellules sans diviser les graphèmes ni convertir les grands offsets en u16.
pub(crate) fn clip_line(line: Line<'static>, left: usize, width: usize) -> Line<'static> {
    if width == 0 {
        return Line::default();
    }
    let mut column = 0;
    let mut spans = Vec::new();
    let right = left.saturating_add(width);
    for grapheme in line.styled_graphemes(Style::default()) {
        let cell_width = Span::raw(grapheme.symbol).width();
        let next = column + cell_width;
        if next > left && column < left {
            append_styled(
                &mut spans,
                &" ".repeat((next - left).min(width)),
                grapheme.style,
            );
        } else if column >= left && next <= right {
            append_styled(&mut spans, grapheme.symbol, grapheme.style);
        }
        column = next;
        if column >= right {
            break;
        }
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        widgets::{Paragraph, Widget},
    };

    #[test]
    fn editable_text_preserves_combining_marks_and_joined_emoji() {
        let source = "é🙂漢e\u{301}👩\u{200d}💻";
        let area = Rect::new(0, 0, 30, 1);
        let mut actual = Buffer::empty(area);
        let mut expected = Buffer::empty(area);
        Paragraph::new(text_with_selection(
            source,
            source.chars().count(),
            None,
            Style::default(),
            Style::default(),
        ))
        .render(area, &mut actual);
        Paragraph::new(source).render(area, &mut expected);
        assert_eq!(actual, expected);
        assert!(actual
            .content
            .iter()
            .any(|cell| cell.symbol() == "👩\u{200d}💻"));
    }

    #[test]
    fn inline_cursor_does_not_split_a_selected_grapheme() {
        let source = "e\u{301}👩\u{200d}💻";
        let rendered = line_with_cursor(
            source,
            1,
            Some(3),
            Style::default(),
            Style::default(),
            Style::default(),
        );
        let area = Rect::new(0, 0, 15, 1);
        let mut actual = Buffer::empty(area);
        let mut expected = Buffer::empty(area);
        Paragraph::new(rendered).render(area, &mut actual);
        Paragraph::new(source).render(area, &mut expected);
        assert_eq!(actual, expected);
    }

    #[test]
    fn viewport_places_cursor_in_terminal_cells_and_scrolls_long_input() {
        let source = "漢🙂e\u{301}";
        let rendered = text_with_selection(source, 2, None, Style::default(), Style::default());
        let (_, cursor) = input_viewport(rendered, source, 2, 10, 1);
        assert_eq!(cursor, Some((4, 0)));

        let source = format!("{}\nligne\né🙂", "漢".repeat(70_000));
        let end = source.chars().count();
        let rendered = text_with_selection(&source, end, None, Style::default(), Style::default());
        let (visible, cursor) = input_viewport(rendered, &source, end, 10, 2);
        assert_eq!(cursor, Some((3, 1)));
        assert_eq!(visible.lines.len(), 2);
        assert_eq!(visible.lines[1].to_string(), "é🙂");

        let long_line = "漢".repeat(70_000);
        let rendered =
            text_with_selection(&long_line, 70_000, None, Style::default(), Style::default());
        let (visible, cursor) = input_viewport(rendered, &long_line, 70_000, 10, 1);
        assert_eq!(cursor, Some((9, 0)));
        assert!(visible.width() <= 10);
    }

    #[test]
    fn empty_input_panel_does_not_place_cursor_outside_the_terminal() {
        let (_, cursor) = input_viewport(Text::raw("text"), "text", 4, 0, 0);
        assert!(cursor.is_none());
    }
}
