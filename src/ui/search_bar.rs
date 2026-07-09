//! Barre de recherche pour la recherche de commits.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::git::search::SearchType;
use crate::state::{selection_range, SearchState};
use crate::ui::theme::current_theme;

pub struct SearchBarRenderContext<'a> {
    pub search_state: &'a SearchState,
    pub area: Rect,
}

/// Rend la barre de recherche quand la recherche est active.
pub fn render(frame: &mut Frame, ctx: SearchBarRenderContext<'_>) {
    let SearchBarRenderContext { search_state, area } = ctx;

    if !search_state.is_active {
        return;
    }

    let theme = current_theme();

    // Construire le texte de recherche avec curseur
    let query_text = &search_state.query;
    let chars: Vec<char> = query_text.chars().collect();
    let cursor_pos = search_state.cursor.min(chars.len());
    let selection = selection_range(cursor_pos, search_state.selection_anchor);

    // Construire la ligne affichée
    let mut spans = vec![];

    // Préfixe de recherche
    spans.push(Span::styled(
        "/",
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    ));

    for (index, character) in chars.iter().enumerate() {
        let style = if index == cursor_pos {
            Style::default()
                .bg(theme.primary)
                .fg(theme.background)
                .add_modifier(Modifier::BOLD)
        } else if selection
            .as_ref()
            .is_some_and(|range| range.contains(&index))
        {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
        } else {
            Style::default()
        };
        spans.push(Span::styled(character.to_string(), style));
    }

    if cursor_pos == chars.len() {
        spans.push(Span::styled(
            " ",
            Style::default()
                .bg(theme.primary)
                .fg(theme.background)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Ajouter le type de recherche
    let type_label = match search_state.search_type {
        SearchType::Message => "msg",
        SearchType::Author => "author",
        SearchType::Hash => "hash",
    };
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        format!("[{}]", type_label),
        Style::default().fg(theme.warning),
    ));

    // Ajouter le compteur de résultats
    if !search_state.results.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!(
                "{}/{}",
                search_state.current_result + 1,
                search_state.results.len()
            ),
            Style::default().fg(theme.success),
        ));
    } else if !query_text.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("0/0", Style::default().fg(theme.error)));
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        )
        .style(Style::default().bg(theme.background));

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tests::render_to_string;

    #[test]
    fn test_search_bar_renders_accented_query_without_invalid_utf8_slice() {
        let mut search_state = SearchState::default();
        search_state.is_active = true;
        search_state.query = "té".to_string();
        search_state.cursor = 2;

        let output = render_to_string(40, 3, |frame| {
            render(
                frame,
                SearchBarRenderContext {
                    search_state: &search_state,
                    area: Rect::new(0, 0, 40, 3),
                },
            );
        });

        assert!(output.contains("té"));
    }
}
