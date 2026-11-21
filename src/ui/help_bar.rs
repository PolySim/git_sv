//! Barre de raccourcis du graphe : les gestes de lecture du panneau actif d'abord.

use ratatui::{layout::Rect, Frame};

use crate::i18n::text;
use crate::state::FocusPanel;
use crate::ui::common::help_bar::KeyHint;

pub struct HelpBarRenderContext {
    pub selected_index: usize,
    pub total_commits: usize,
    pub focus: FocusPanel,
    pub filter_active: bool,
    pub is_merging: bool,
    pub area: Rect,
}

pub fn render(frame: &mut Frame, ctx: HelpBarRenderContext) {
    let mut keys = vec![
        KeyHint::new("?", text("aide", "help")),
        KeyHint::new("a", text("marquer A", "mark A")),
        KeyHint::new("D", "A → B"),
        KeyHint::new("Ctrl+A", text("effacer A", "clear A")),
    ];
    if ctx.filter_active {
        keys.push(KeyHint::new(
            "Ctrl+R",
            text("effacer filtres", "clear filters"),
        ));
    }
    if ctx.is_merging {
        keys.push(KeyHint::new("A", text("annuler fusion", "abort merge")));
    }
    match ctx.focus {
        FocusPanel::Graph => keys.extend([
            KeyHint::new("j/k", text("commits", "commits")),
            KeyHint::new("Enter", text("fichiers", "files")),
            KeyHint::new("/", text("rechercher", "search")),
            KeyHint::new("F", text("filtres", "filters")),
        ]),
        FocusPanel::BottomLeft => keys.extend([
            KeyHint::new("j/k", text("fichiers", "files")),
            KeyHint::new(text("Espace", "Space"), "diff"),
            KeyHint::new("z", text("plein écran", "fullscreen")),
        ]),
        FocusPanel::BottomRight => keys.extend([
            KeyHint::new("j/k", text("défiler", "scroll")),
            KeyHint::new("n/N", text("hunks", "hunks")),
            KeyHint::new("v", text("présentation", "layout")),
            KeyHint::new("z", text("plein écran", "fullscreen")),
        ]),
    }
    keys.extend([
        KeyHint::new("Tab", text("panneau", "panel")),
        KeyHint::new("q", text("quitter", "quit")),
    ]);
    let position = ctx.selected_index.saturating_add(1).min(ctx.total_commits);
    let counter = format!("{position}/{}", ctx.total_commits);
    crate::ui::common::help_bar::render(frame, ctx.area, &keys, Some(&counter));
}
