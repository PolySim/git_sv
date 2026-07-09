//! Popup de filtrage pour le graph de commits.

use crate::i18n::text;
use crate::ui::common::centered_rect;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::state::{selection_range, FilterField, FilterPopupState, GraphFilter};
use crate::ui::theme::current_theme;

pub struct FilterPopupRenderContext<'a> {
    pub popup_state: &'a FilterPopupState,
    pub current_filter: &'a GraphFilter,
    pub area: Rect,
}

struct FilterFieldRenderContext<'a> {
    label: &'a str,
    value: &'a str,
    is_selected: bool,
    cursor: usize,
    selection_anchor: Option<usize>,
    area: Rect,
    theme: &'a crate::ui::theme::Theme,
}

/// Rend le popup de filtre si ouvert.
pub fn render(frame: &mut Frame, ctx: FilterPopupRenderContext<'_>) {
    let FilterPopupRenderContext {
        popup_state,
        current_filter,
        area,
    } = ctx;

    if !popup_state.is_open {
        return;
    }

    let theme = current_theme();

    // Zone centrale pour le popup
    let popup_area = centered_rect(70, 60, area);

    // Clear le fond
    frame.render_widget(Clear, popup_area);

    // Bordure avec titre
    let is_active = current_filter.is_active();
    let title = if is_active {
        text("Filtres de commits (actifs)", "Commit filters (active)")
    } else {
        text("Filtres de commits", "Commit filters")
    };

    let border_style = if is_active {
        Style::default().fg(theme.warning)
    } else {
        Style::default().fg(theme.primary)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.background));

    frame.render_widget(block, popup_area);

    // Layout interne
    let inner = popup_area.inner(Margin::new(2, 1));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Description
            Constraint::Length(1), // Espacement
            Constraint::Length(3), // Auteur
            Constraint::Length(3), // Date de début
            Constraint::Length(3), // Date de fin
            Constraint::Length(3), // Chemin
            Constraint::Length(3), // Message
            Constraint::Length(2), // Espacement
            Constraint::Length(1), // Aide
        ])
        .split(inner);

    // Description
    let desc = Paragraph::new(text(
        "Filtrer les commits affiches dans le graphe",
        "Filter commits shown in the graph",
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(theme.text_secondary));
    frame.render_widget(desc, chunks[0]);

    // Champs de filtre
    render_filter_field(
        frame,
        FilterFieldRenderContext {
            label: text("Auteur", "Author"),
            value: &popup_state.author_input,
            is_selected: popup_state.selected_field == FilterField::Author,
            cursor: popup_state.current_cursor(),
            selection_anchor: popup_state.current_selection_anchor(),
            area: chunks[2],
            theme,
        },
    );

    render_filter_field(
        frame,
        FilterFieldRenderContext {
            label: text("Date debut (YYYY-MM-DD)", "Start date (YYYY-MM-DD)"),
            value: &popup_state.date_from_input,
            is_selected: popup_state.selected_field == FilterField::DateFrom,
            cursor: popup_state.current_cursor(),
            selection_anchor: popup_state.current_selection_anchor(),
            area: chunks[3],
            theme,
        },
    );

    render_filter_field(
        frame,
        FilterFieldRenderContext {
            label: text("Date fin (YYYY-MM-DD)", "End date (YYYY-MM-DD)"),
            value: &popup_state.date_to_input,
            is_selected: popup_state.selected_field == FilterField::DateTo,
            cursor: popup_state.current_cursor(),
            selection_anchor: popup_state.current_selection_anchor(),
            area: chunks[4],
            theme,
        },
    );

    render_filter_field(
        frame,
        FilterFieldRenderContext {
            label: text("Chemin", "Path"),
            value: &popup_state.path_input,
            is_selected: popup_state.selected_field == FilterField::Path,
            cursor: popup_state.current_cursor(),
            selection_anchor: popup_state.current_selection_anchor(),
            area: chunks[5],
            theme,
        },
    );

    render_filter_field(
        frame,
        FilterFieldRenderContext {
            label: text("Message contient", "Message contains"),
            value: &popup_state.message_input,
            is_selected: popup_state.selected_field == FilterField::Message,
            cursor: popup_state.current_cursor(),
            selection_anchor: popup_state.current_selection_anchor(),
            area: chunks[6],
            theme,
        },
    );

    // Aide en bas
    let help_text = text(
        "⌘/⌥/⇧+←→: editer | ⌘Z:annuler | Ctrl+R:effacer | Entree:appliquer",
        "Cmd/Alt/Shift+←→: edit | Cmd+Z:undo | Ctrl+R:clear | Enter:apply",
    );
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text_secondary));
    frame.render_widget(help, chunks[8]);
}

/// Rend un champ de filtre individuel.
fn render_filter_field(frame: &mut Frame, ctx: FilterFieldRenderContext<'_>) {
    let FilterFieldRenderContext {
        label,
        value,
        is_selected,
        cursor,
        selection_anchor,
        area,
        theme,
    } = ctx;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Label
    let label_style = if is_selected {
        Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_secondary)
    };

    let label_span = Span::styled(format!("{}: ", label), label_style);
    let label_line = Line::from(vec![label_span]);
    let label_para = Paragraph::new(label_line);
    frame.render_widget(label_para, chunks[0]);

    // Valeur avec fond
    let (bg_color, fg_color) = if is_selected {
        (theme.selection_bg, theme.selection_fg)
    } else {
        // Utiliser text_normal pour une meilleure lisibilité (White sur Black en sombre, Black sur White en clair)
        (theme.background, theme.text_normal)
    };

    let value_style = if value.is_empty() && is_selected {
        Style::default().fg(theme.text_secondary).bg(bg_color)
    } else {
        Style::default().fg(fg_color).bg(bg_color)
    };

    let prefix = if is_selected { "> " } else { "  " };
    let value_para = if is_selected {
        let chars: Vec<char> = value.chars().collect();
        let cursor = cursor.min(chars.len());
        let selection = selection_range(cursor, selection_anchor);
        let mut spans = vec![Span::styled(prefix, value_style)];

        for (index, character) in chars.iter().enumerate() {
            let style = if index == cursor {
                Style::default().fg(theme.background).bg(theme.warning)
            } else if selection
                .as_ref()
                .is_some_and(|range| range.contains(&index))
            {
                Style::default().fg(theme.background).bg(theme.primary)
            } else {
                value_style
            };
            spans.push(Span::styled(character.to_string(), style));
        }
        if cursor == chars.len() {
            spans.push(Span::styled(
                " ",
                Style::default().fg(theme.background).bg(theme.warning),
            ));
        }
        Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false })
    } else {
        let display_value = if value.is_empty() {
            text(" (vide) ", " (empty) ")
        } else {
            value
        };
        Paragraph::new(format!("{}{}", prefix, display_value))
            .style(value_style)
            .wrap(Wrap { trim: false })
    };

    frame.render_widget(value_para, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tests::render_to_string;

    #[test]
    fn test_filter_popup_displays_current_input_and_reset_legend() {
        let popup_state = FilterPopupState {
            is_open: true,
            selected_field: FilterField::Author,
            author_input: "Alice".to_string(),
            ..FilterPopupState::default()
        };

        let output = render_to_string(100, 30, |frame| {
            render(
                frame,
                FilterPopupRenderContext {
                    popup_state: &popup_state,
                    current_filter: &GraphFilter::new(),
                    area: Rect::new(0, 0, 100, 30),
                },
            );
        });

        assert!(output.contains("Alice"));
        assert!(output.contains("Ctrl+R"));
    }
}
