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

    for (index, character) in text.chars().enumerate() {
        if character == '\n' {
            lines.push(Line::default());
            continue;
        }

        let style = if selection
            .as_ref()
            .is_some_and(|range| range.contains(&index))
        {
            selection_style
        } else {
            normal_style
        };
        lines
            .last_mut()
            .expect("au moins une ligne")
            .spans
            .push(Span::styled(character.to_string(), style));
    }

    Text::from(lines)
}
