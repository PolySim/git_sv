use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::BottomLeftMode;
use crate::ui::theme::current_theme;

/// Rend la barre d'aide persistante en bas de l'écran.
pub fn render(
    frame: &mut Frame,
    selected_index: usize,
    total_commits: usize,
    bottom_left_mode: BottomLeftMode,
    filter_active: bool,
    area: Rect,
) {
    let theme = current_theme();

    // Déterminer les touches à afficher.
    let mut keys = vec![
        ("j/k", "naviguer"),
        ("Enter", "détail"),
        ("b", "branches"),
        ("c", "commit"),
        ("s", "stash"),
        ("m", "merge"),
        ("P", "push"),
    ];

    // Ajouter le contexte du panneau bas.
    match bottom_left_mode {
        BottomLeftMode::CommitFiles | BottomLeftMode::Files => keys.push(("Tab", "fichiers")),
        BottomLeftMode::WorkingDir | BottomLeftMode::Parents => keys.push(("Tab", "commit")),
    }

    // Ajouter le raccourci pour effacer les filtres s'ils sont actifs
    if filter_active {
        keys.push(("Ctrl+R", "effacer filtres"));
    }

    keys.extend(vec![("r", "rafraîchir"), ("?", "aide"), ("q", "quitter")]);

    // Construire la ligne avec les touches formatées.
    let mut spans = build_help_spans(&keys, theme);

    // Ajouter le compteur de commits à droite.
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        format!("{}/{}", selected_index + 1, total_commits),
        Style::default().fg(theme.text_secondary),
    ));

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.border_inactive)),
    );

    frame.render_widget(paragraph, area);
}

/// Construit les spans pour la barre d'aide.
fn build_help_spans<'a>(
    keys: &'a [(&'a str, &'a str)],
    theme: &crate::ui::theme::Theme,
) -> Vec<Span<'a>> {
    let mut spans: Vec<Span<'a>> = Vec::with_capacity(keys.len() * 3);

    for (i, (key, desc)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }

        // Touche en cyan + bold.
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

        // Description en couleur normale.
        spans.push(Span::raw(":"));
        spans.push(Span::styled(*desc, Style::default().fg(theme.text_normal)));
    }

    spans
}
