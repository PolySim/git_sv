//! Overlay d'aide complète (touche `?`), affiche tous les raccourcis clavier.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::common::centered_rect;
use crate::ui::keybindings;
use crate::ui::theme::current_theme;

pub struct HelpOverlayRenderContext {
    pub area: Rect,
}

/// Rend l'overlay d'aide complet centré sur l'écran.
pub fn render(frame: &mut Frame, ctx: HelpOverlayRenderContext) {
    let HelpOverlayRenderContext { area } = ctx;

    let theme = current_theme();
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
                .border_style(Style::default().fg(theme.primary)),
        )
        .style(Style::default().bg(theme.background).fg(theme.text_normal))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}

/// Construit le contenu textuel de l'overlay d'aide.
fn build_help_content() -> Vec<Line<'static>> {
    let theme = current_theme();
    vec![
        Line::from(""),
        // ── Navigation ──
        section_header("Navigation"),
        separator(),
        key_line_multi(keybindings::navigation::DOWN, "Commit suivant"),
        key_line_multi(keybindings::navigation::UP, "Commit précédent"),
        key_line_multi(keybindings::navigation::TOP, "Premier commit"),
        key_line_multi(keybindings::navigation::BOTTOM, "Dernier commit"),
        key_line_multi(keybindings::navigation::PAGE_DOWN, "Page suivante"),
        key_line_multi(keybindings::navigation::PAGE_UP, "Page précédente"),
        key_line(keybindings::navigation::SWITCH_PANEL, "Basculer panneaux"),
        key_line("Enter", "Contextuel (sélectionner/valider/plein écran)"),
        key_line("Espace", "Ouvrir le panneau diff"),
        Line::from(""),
        // ── Vues ──
        section_header("Vues"),
        separator(),
        key_line(keybindings::global::VIEW_GRAPH, "Vue Graph"),
        key_line(keybindings::global::VIEW_STAGING, "Vue Staging"),
        key_line(keybindings::global::VIEW_BRANCHES, "Vue Branches"),
        key_line(
            keybindings::global::VIEW_CONFLICTS,
            "Vue Conflits (si actifs)",
        ),
        Line::from(""),
        // ── Actions Git ──
        section_header("Actions Git"),
        separator(),
        key_line(keybindings::git_actions::COMMIT, "Nouveau commit"),
        key_line(keybindings::git_actions::STASH, "Stash"),
        key_line(keybindings::git_actions::MERGE, "Merge"),
        key_line(keybindings::git_actions::BRANCH_PANEL, "Panneau branches"),
        key_line(keybindings::git_actions::PUSH, "Push"),
        key_line(keybindings::git_actions::FORCE_PUSH, "Force push"),
        key_line(keybindings::git_actions::PULL, "Pull"),
        key_line(keybindings::git_actions::FETCH, "Fetch"),
        key_line(keybindings::git_actions::CHERRY_PICK, "Cherry-pick"),
        key_line(keybindings::git_actions::BLAME, "Blame du fichier"),
        key_line(keybindings::git_actions::RESET, "Reset"),
        Line::from(""),
        // ── Recherche & Filtre ──
        section_header("Recherche & Filtre"),
        separator(),
        key_line(keybindings::search::OPEN, "Ouvrir la recherche"),
        key_line(keybindings::search::NEXT, "Résultat suivant"),
        key_line(keybindings::search::PREVIOUS, "Résultat précédent"),
        key_line(keybindings::search::FILTER, "Filtre avancé"),
        Line::from(""),
        // ── Interface ──
        section_header("Interface"),
        separator(),
        key_line(
            keybindings::diff::TOGGLE_VIEW,
            "Toggle diff (unified/split)",
        ),
        key_line(keybindings::global::REFRESH, "Rafraîchir"),
        key_line(keybindings::global::COPY, "Copier dans le clipboard"),
        key_line_multi(keybindings::global::QUIT, "Quitter"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Esc ou ? pour fermer",
            Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::ITALIC),
        )]),
    ]
}

fn section_header(title: &str) -> Line<'static> {
    let theme = current_theme();
    Line::from(vec![Span::styled(
        title.to_string(),
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(theme.warning),
    )])
}

fn separator() -> Line<'static> {
    Line::from("─".repeat(40))
}

fn key_line(key: &str, desc: &str) -> Line<'static> {
    let theme = current_theme();
    let padding = 16usize.saturating_sub(key.len());
    Line::from(vec![
        Span::styled(key.to_string(), Style::default().fg(theme.primary)),
        Span::raw(format!("{}{}", " ".repeat(padding), desc)),
    ])
}

fn key_line_multi(keys: &[&str], desc: &str) -> Line<'static> {
    let theme = current_theme();
    let keys_str = keys.join(" / ");
    let padding = 16usize.saturating_sub(keys_str.len());
    Line::from(vec![
        Span::styled(keys_str, Style::default().fg(theme.primary)),
        Span::raw(format!("{}{}", " ".repeat(padding), desc)),
    ])
}
