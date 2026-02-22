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
        // ── Navigation ──
        section_header("Navigation"),
        separator(),
        key_line("j / ↓", "Commit suivant"),
        key_line("k / ↑", "Commit précédent"),
        key_line("g / Home", "Premier commit"),
        key_line("G / End", "Dernier commit"),
        key_line("Ctrl+D / PgDn", "Page suivante"),
        key_line("Ctrl+U / PgUp", "Page précédente"),
        key_line("Enter", "Détail / action"),
        key_line("Tab", "Basculer panneaux"),
        Line::from(""),
        // ── Vues ──
        section_header("Vues"),
        separator(),
        key_line("1", "Vue Graph"),
        key_line("2", "Vue Staging"),
        key_line("3", "Vue Branches"),
        key_line("4", "Vue Conflits (si actifs)"),
        Line::from(""),
        // ── Actions Git ──
        section_header("Actions Git"),
        separator(),
        key_line("c", "Nouveau commit"),
        key_line("s", "Stash"),
        key_line("m", "Merge"),
        key_line("b", "Panneau branches"),
        key_line("P", "Push"),
        key_line("p", "Pull"),
        key_line("f", "Fetch"),
        key_line("x", "Cherry-pick"),
        key_line("B", "Blame du fichier"),
        Line::from(""),
        // ── Recherche & Filtre ──
        section_header("Recherche & Filtre"),
        separator(),
        key_line("/", "Ouvrir la recherche"),
        key_line("n / N", "Résultat suivant / précédent"),
        key_line("F", "Filtre avancé"),
        Line::from(""),
        // ── Interface ──
        section_header("Interface"),
        separator(),
        key_line("v", "Toggle diff (unified/split)"),
        key_line("r", "Rafraîchir"),
        key_line("y", "Copier dans le clipboard"),
        key_line("q", "Quitter"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Esc ou ? pour fermer",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]),
    ]
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(vec![Span::styled(
        title.to_string(),
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Yellow),
    )])
}

fn separator() -> Line<'static> {
    Line::from("─".repeat(40))
}

fn key_line(key: &str, desc: &str) -> Line<'static> {
    let padding = 16usize.saturating_sub(key.len());
    Line::from(vec![
        Span::styled(key.to_string(), Style::default().fg(Color::Cyan)),
        Span::raw(format!("{}{}", " ".repeat(padding), desc)),
    ])
}
