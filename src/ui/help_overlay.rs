use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::common::centered_rect;

/// Rend l'overlay d'aide complet centré sur l'écran.
pub fn render(frame: &mut Frame, area: Rect) {
    // Créer une zone centrale pour le popup (70% largeur, 80% hauteur).
    let popup_area = centered_rect(70, 80, area);

    // Effacer l'arrière-plan derrière le popup.
    frame.render_widget(Clear, popup_area);

    // Construire le contenu de l'aide.
    let content = build_help_content();

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Aide ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}

/// Construit le contenu textuel de l'overlay d'aide.
fn build_help_content() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from("─".repeat(40)),
        Line::from(vec![
            Span::styled("j / ↓", Style::default().fg(Color::Cyan)),
            Span::raw("       Commit suivant"),
        ]),
        Line::from(vec![
            Span::styled("k / ↑", Style::default().fg(Color::Cyan)),
            Span::raw("       Commit précédent"),
        ]),
        Line::from(vec![
            Span::styled("g", Style::default().fg(Color::Cyan)),
            Span::raw("           Premier commit"),
        ]),
        Line::from(vec![
            Span::styled("G", Style::default().fg(Color::Cyan)),
            Span::raw("           Dernier commit"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions Git",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from("─".repeat(40)),
        Line::from(vec![
            Span::styled("c", Style::default().fg(Color::Cyan)),
            Span::raw("           Nouveau commit"),
        ]),
        Line::from(vec![
            Span::styled("s", Style::default().fg(Color::Cyan)),
            Span::raw("           Stash"),
        ]),
        Line::from(vec![
            Span::styled("m", Style::default().fg(Color::Cyan)),
            Span::raw("           Merge"),
        ]),
        Line::from(vec![
            Span::styled("b", Style::default().fg(Color::Cyan)),
            Span::raw("           Branches"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Interface",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from("─".repeat(40)),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw("         Basculer panneaux"),
        ]),
        Line::from(vec![
            Span::styled("r", Style::default().fg(Color::Cyan)),
            Span::raw("           Rafraîchir"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Sélection & Clipboard",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from("─".repeat(40)),
        Line::from(vec![
            Span::styled("Shift+clic", Style::default().fg(Color::Cyan)),
            Span::raw("    Sélection native du terminal"),
        ]),
        Line::from(vec![
            Span::styled("y", Style::default().fg(Color::Cyan)),
            Span::raw("           Copier dans le clipboard"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Esc ou ? pour fermer",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]),
    ]
}
