use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::BottomLeftMode;

/// Rend la barre d'aide persistante en bas de l'écran.
pub fn render(
    frame: &mut Frame,
    selected_index: usize,
    total_commits: usize,
    bottom_left_mode: BottomLeftMode,
    area: Rect,
) {
    // Déterminer les touches à afficher.
    let mut keys = vec![
        ("j/k", "naviguer"),
        ("Enter", "détail"),
        ("b", "branches"),
        ("c", "commit"),
        ("s", "stash"),
        ("m", "merge"),
    ];

    // Ajouter le contexte du panneau bas.
    match bottom_left_mode {
        BottomLeftMode::CommitFiles => keys.push(("Tab", "fichiers")),
        BottomLeftMode::WorkingDir => keys.push(("Tab", "commit")),
    }

    keys.extend(vec![("r", "rafraîchir"), ("?", "aide"), ("q", "quitter")]);

    // Construire la ligne avec les touches formatées.
    let mut spans = build_help_spans(&keys);

    // Ajouter le compteur de commits à droite.
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        format!("{}/{}", selected_index + 1, total_commits),
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, area);
}

/// Construit les spans pour la barre d'aide.
fn build_help_spans<'a>(keys: &'a [(&'a str, &'a str)]) -> Vec<Span<'a>> {
    let mut spans: Vec<Span<'a>> = Vec::with_capacity(keys.len() * 3);

    for (i, (key, desc)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }

        // Touche en cyan + bold.
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

        // Description en blanc.
        spans.push(Span::raw(":"));
        spans.push(Span::styled(*desc, Style::default().fg(Color::White)));
    }

    spans
}
