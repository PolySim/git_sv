//! Composant de sélection du type de reset.

use crate::i18n::{text, text_owned};
use crate::state::ResetPickerState;
use crate::ui::common::centered_rect;
use crate::ui::theme::current_theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct ResetPickerRenderContext<'a> {
    pub state: &'a ResetPickerState,
    pub area: Rect,
}

/// Rend le picker de reset en overlay.
pub fn render(frame: &mut Frame, ctx: ResetPickerRenderContext<'_>) {
    let ResetPickerRenderContext { state, area } = ctx;

    let theme = current_theme();
    // Calculer la zone centrale pour le popup
    let popup_area = centered_rect(60, 40, area);

    // Effacer la zone sous le popup
    frame.render_widget(Clear, popup_area);

    // Construire le titre
    let title = text_owned(
        format!(" Reset vers {} ", state.short_hash),
        format!(" Reset to {} ", state.short_hash),
    );

    // Message du commit (tronqué si trop long)
    let commit_msg = if state.commit_message.len() > 50 {
        format!("{}...", &state.commit_message[..50])
    } else {
        state.commit_message.clone()
    };

    // Styles pour les options
    let soft_style = if state.is_soft_selected() {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_secondary)
    };

    let hard_style = if state.is_hard_selected() {
        Style::default()
            .fg(theme.error)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_secondary)
    };

    let soft_indicator = if state.is_soft_selected() {
        "▶ "
    } else {
        "  "
    };
    let hard_indicator = if state.is_hard_selected() {
        "▶ "
    } else {
        "  "
    };

    // Contenu du popup
    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(text("Commit: ", "Commit: ")),
            Span::styled(commit_msg, Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(text("Choisissez le type de reset :", "Choose reset type:")),
        Line::from(""),
        Line::from(vec![
            Span::raw(soft_indicator),
            Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(text(" - Soft ", " - Soft ")),
            Span::styled(
                text(
                    "(garde les modifications stagees)",
                    "(keeps staged changes)",
                ),
                soft_style,
            ),
        ]),
        Line::from(vec![
            Span::raw(hard_indicator),
            Span::styled("h", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(text(" - Hard ", " - Hard ")),
            Span::styled(
                text("(perd toutes les modifications)", "(loses all changes)"),
                hard_style,
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "ENTER",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(text(" - Confirmer  ", " - Confirm  ")),
            Span::styled(
                "ESC",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(text(" - Annuler", " - Cancel")),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.warning)),
        )
        .style(Style::default().bg(theme.background).fg(theme.text_normal))
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, popup_area);
}
