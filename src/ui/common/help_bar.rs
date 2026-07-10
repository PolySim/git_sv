//! Barre d'aide contextuelle, lisible et adaptée à la largeur disponible.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::current_theme;

/// Raccourci et libellé d'action associés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyHint<'a> {
    pub key: &'a str,
    pub label: &'a str,
}

impl<'a> KeyHint<'a> {
    pub const fn new(key: &'a str, label: &'a str) -> Self {
        Self { key, label }
    }
}

/// Rend les raccourcis prioritaires qui tiennent dans la largeur disponible.
pub fn render(frame: &mut Frame, area: Rect, hints: &[KeyHint<'_>], trailing: Option<&str>) {
    let theme = current_theme();
    let trailing_width = trailing.map(display_width).unwrap_or(0);
    let trailing_gap = usize::from(trailing.is_some()) * 2;
    let available = usize::from(area.width).saturating_sub(trailing_width + trailing_gap);
    let visible = fit_hints(hints, available);
    let mut spans = Vec::with_capacity(visible.len() * 4 + 2);

    for (index, hint) in visible.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            hint.key,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(":{}", hint.label),
            Style::default().fg(theme.text_secondary),
        ));
    }

    if let Some(trailing) = trailing {
        let used: usize = spans
            .iter()
            .map(|span| display_width(span.content.as_ref()))
            .sum();
        let padding = usize::from(area.width).saturating_sub(used + trailing_width);
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(
            trailing.to_string(),
            Style::default().fg(theme.text_secondary),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(theme.surface))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.border_inactive))
                .style(Style::default().bg(theme.surface)),
        );
    frame.render_widget(paragraph, area);
}

fn fit_hints<'a>(hints: &'a [KeyHint<'a>], available: usize) -> &'a [KeyHint<'a>] {
    let mut used = 0;
    let count = hints
        .iter()
        .take_while(|hint| {
            let separator = usize::from(used > 0) * 2;
            let width = display_width(hint.key) + 1 + display_width(hint.label);
            if used + separator + width > available {
                return false;
            }
            used += separator + width;
            true
        })
        .count();
    &hints[..count]
}

fn display_width(value: &str) -> usize {
    Line::from(value).width()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fit_hints_keeps_priority_order() {
        let hints = [
            KeyHint::new("j/k", "navigate"),
            KeyHint::new("Enter", "details"),
            KeyHint::new("P", "push"),
        ];

        assert_eq!(fit_hints(&hints, 24), &hints[..1]);
    }
}
