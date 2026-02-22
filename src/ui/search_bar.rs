//! Barre de recherche pour la recherche de commits.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::git::search::SearchType;
use crate::state::SearchState;
use crate::ui::theme::current_theme;

/// Rend la barre de recherche quand la recherche est active.
pub fn render(frame: &mut Frame, search_state: &SearchState, area: Rect) {
    if !search_state.is_active {
        return;
    }

    let theme = current_theme();

    // Construire le texte de recherche avec curseur
    let query_text = &search_state.query;
    let cursor_pos = search_state.cursor;

    // Construire la ligne affichée
    let mut spans = vec![];

    // Préfixe de recherche
    spans.push(Span::styled(
        "/",
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    ));

    // Texte de recherche avant le curseur
    if cursor_pos > 0 && cursor_pos <= query_text.len() {
        spans.push(Span::raw(&query_text[..cursor_pos]));
    }

    // Caractère sous le curseur (ou espace si fin)
    let cursor_char = query_text.chars().nth(cursor_pos).unwrap_or(' ');
    spans.push(Span::styled(
        cursor_char.to_string(),
        Style::default()
            .bg(theme.primary)
            .fg(theme.background)
            .add_modifier(Modifier::BOLD),
    ));

    // Texte après le curseur
    if cursor_pos < query_text.len() {
        spans.push(Span::raw(&query_text[cursor_pos + 1..]));
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
            format!("{}/{}", search_state.current_result + 1, search_state.results.len()),
            Style::default().fg(theme.success),
        ));
    } else if !query_text.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            "0/0",
            Style::default().fg(theme.error),
        ));
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

/// Calcule la hauteur nécessaire pour la barre de recherche.
pub fn height() -> u16 {
    3 // 1 ligne de contenu + 2 bordures
}
