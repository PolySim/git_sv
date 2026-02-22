//! Popup de filtrage pour le graph de commits.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::state::{FilterField, FilterPopupState, GraphFilter};
use crate::ui::theme::current_theme;

/// Rend le popup de filtre si ouvert.
pub fn render(
    frame: &mut Frame,
    popup_state: &FilterPopupState,
    current_filter: &GraphFilter,
    area: Rect,
) {
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
        "Filtres de commits (actifs)"
    } else {
        "Filtres de commits"
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
    let desc = Paragraph::new("Filtrer les commits affichés dans le graph")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text_secondary));
    frame.render_widget(desc, chunks[0]);

    // Champs de filtre
    render_filter_field(
        frame,
        "Auteur",
        &popup_state.author_input,
        popup_state.selected_field == FilterField::Author,
        chunks[2],
        theme,
    );

    render_filter_field(
        frame,
        "Date début (YYYY-MM-DD)",
        &popup_state.date_from_input,
        popup_state.selected_field == FilterField::DateFrom,
        chunks[3],
        theme,
    );

    render_filter_field(
        frame,
        "Date fin (YYYY-MM-DD)",
        &popup_state.date_to_input,
        popup_state.selected_field == FilterField::DateTo,
        chunks[4],
        theme,
    );

    render_filter_field(
        frame,
        "Chemin",
        &popup_state.path_input,
        popup_state.selected_field == FilterField::Path,
        chunks[5],
        theme,
    );

    render_filter_field(
        frame,
        "Message contient",
        &popup_state.message_input,
        popup_state.selected_field == FilterField::Message,
        chunks[6],
        theme,
    );

    // Aide en bas
    let help_text = if is_active {
        "Tab/↑↓: changer champ | Entrée: appliquer | Échap: fermer | Ctrl+R: effacer"
    } else {
        "Tab/↑↓: changer champ | Entrée: appliquer | Échap: fermer"
    };
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text_secondary));
    frame.render_widget(help, chunks[8]);

    // Rendre le bloc par-dessus
    frame.render_widget(block, popup_area);
}

/// Rend un champ de filtre individuel.
fn render_filter_field(
    frame: &mut Frame,
    label: &str,
    value: &str,
    is_selected: bool,
    area: Rect,
    theme: &crate::ui::theme::Theme,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
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

    let display_value = if value.is_empty() { " (vide) " } else { value };

    let value_style = if value.is_empty() && is_selected {
        Style::default().fg(theme.text_secondary).bg(bg_color)
    } else {
        Style::default().fg(fg_color).bg(bg_color)
    };

    let value_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_selected {
            Style::default().fg(theme.warning)
        } else {
            Style::default().fg(theme.border_inactive)
        });

    let value_para = Paragraph::new(display_value)
        .style(value_style)
        .block(value_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(value_para, chunks[1]);
}

/// Calcule un rectangle centré.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
