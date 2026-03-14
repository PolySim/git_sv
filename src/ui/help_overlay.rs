//! Overlay d'aide complète (touche `?`), affiche tous les raccourcis clavier.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::i18n::{text, text_owned};
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
                .title(text(" Aide ", " Help "))
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
        section_header(text("Navigation", "Navigation")),
        separator(),
        key_line_multi(
            keybindings::navigation::DOWN,
            text("Commit suivant", "Next commit"),
        ),
        key_line_multi(
            keybindings::navigation::UP,
            text("Commit precedent", "Previous commit"),
        ),
        key_line_multi(
            keybindings::navigation::TOP,
            text("Premier commit", "First commit"),
        ),
        key_line_multi(
            keybindings::navigation::BOTTOM,
            text("Dernier commit", "Last commit"),
        ),
        key_line_multi(
            keybindings::navigation::PAGE_DOWN,
            text("Page suivante", "Next page"),
        ),
        key_line_multi(
            keybindings::navigation::PAGE_UP,
            text("Page precedente", "Previous page"),
        ),
        key_line(
            keybindings::navigation::SWITCH_PANEL,
            text("Basculer panneaux", "Switch panels"),
        ),
        key_line(
            "Enter",
            text(
                "Contextuel (selectionner/valider/plein ecran)",
                "Contextual (select/confirm/fullscreen)",
            ),
        ),
        key_line("Espace", text("Ouvrir le panneau diff", "Open diff panel")),
        Line::from(""),
        // ── Vues ──
        section_header(text("Vues", "Views")),
        separator(),
        key_line(
            keybindings::global::VIEW_GRAPH,
            text("Vue Graph", "Graph view"),
        ),
        key_line(
            keybindings::global::VIEW_STAGING,
            text("Vue Staging", "Staging view"),
        ),
        key_line(
            keybindings::global::VIEW_BRANCHES,
            text("Vue Branches", "Branches view"),
        ),
        key_line(
            keybindings::global::VIEW_CONFLICTS,
            text("Vue Conflits (si actifs)", "Conflicts view (if active)"),
        ),
        Line::from(""),
        // ── Actions Git ──
        section_header(text("Actions Git", "Git Actions")),
        separator(),
        key_line(
            keybindings::git_actions::COMMIT,
            text("Nouveau commit", "New commit"),
        ),
        key_line(keybindings::git_actions::STASH, text("Stash", "Stash")),
        key_line(keybindings::git_actions::MERGE, text("Merge", "Merge")),
        key_line(
            keybindings::git_actions::BRANCHES,
            text("Vue branches", "Branches view"),
        ),
        key_line(keybindings::git_actions::PUSH, text("Push", "Push")),
        key_line(
            keybindings::git_actions::FORCE_PUSH,
            text("Force push", "Force push"),
        ),
        key_line(keybindings::git_actions::PULL, text("Pull", "Pull")),
        key_line(keybindings::git_actions::FETCH, text("Fetch", "Fetch")),
        key_line(
            keybindings::git_actions::CHERRY_PICK,
            text("Cherry-pick", "Cherry-pick"),
        ),
        key_line(
            keybindings::git_actions::BLAME,
            text("Blame du fichier", "File blame"),
        ),
        key_line(keybindings::git_actions::RESET, text("Reset", "Reset")),
        Line::from(""),
        // ── Recherche & Filtre ──
        section_header(text("Recherche & Filtre", "Search & Filter")),
        separator(),
        key_line(
            keybindings::search::OPEN,
            text("Ouvrir la recherche", "Open search"),
        ),
        key_line(
            keybindings::search::NEXT,
            text("Resultat suivant", "Next result"),
        ),
        key_line(
            keybindings::search::PREVIOUS,
            text("Resultat precedent", "Previous result"),
        ),
        key_line(
            keybindings::search::FILTER,
            text("Filtre avance", "Advanced filter"),
        ),
        Line::from(""),
        // ── Interface ──
        section_header(text("Interface", "Interface")),
        separator(),
        key_line(
            keybindings::diff::TOGGLE_VIEW,
            text(
                "Basculer diff (unifie/split)",
                "Toggle diff (unified/split)",
            ),
        ),
        key_line(keybindings::global::REFRESH, text("Rafraichir", "Refresh")),
        key_line(
            keybindings::global::COPY,
            text("Copier dans le presse-papiers", "Copy to clipboard"),
        ),
        key_line_multi(keybindings::global::QUIT, text("Quitter", "Quit")),
        Line::from(""),
        Line::from(vec![Span::styled(
            text_owned("Esc ou ? pour fermer", "Esc or ? to close"),
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
