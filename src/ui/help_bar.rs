//! Barre d'aide contextuelle en bas de l'écran (raccourcis clavier).

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::BottomLeftMode;
use crate::ui::theme::current_theme;

pub struct HelpBarRenderContext {
    pub selected_index: usize,
    pub total_commits: usize,
    pub bottom_left_mode: BottomLeftMode,
    pub filter_active: bool,
    pub is_merging: bool,
    pub area: Rect,
}

/// Rend la barre d'aide persistante en bas de l'écran.
pub fn render(frame: &mut Frame, ctx: HelpBarRenderContext) {
    let HelpBarRenderContext {
        selected_index,
        total_commits,
        bottom_left_mode,
        filter_active,
        is_merging,
        area,
    } = ctx;

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
        ("Ctrl+P", "force push"),
    ];

    // Ajouter abort merge si un merge est en cours.
    if is_merging {
        keys.push(("A", "abort merge"));
    }

    // Ajouter le contexte du panneau bas.
    match bottom_left_mode {
        BottomLeftMode::Files => {
            keys.push(("Tab", "fichiers"));
            keys.push(("Espace", "diff"));
            keys.push(("Enter", "plein ecran"));
        }
        BottomLeftMode::Parents => keys.push(("Tab", "commit")),
    }

    // Ajouter le raccourci pour effacer les filtres s'ils sont actifs
    if filter_active {
        keys.push(("Ctrl+R", "effacer filtres"));
    }

    keys.extend(vec![("r", "rafraîchir"), ("?", "aide"), ("q", "quitter")]);

    keys.push(("clic", "focus/select"));
    keys.push(("molette", "scroll"));

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
