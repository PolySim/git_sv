//! Barre d'aide contextuelle en bas de l'écran (raccourcis clavier).

use ratatui::{layout::Rect, Frame};

use crate::i18n::text;
use crate::state::BottomLeftMode;
use crate::ui::common::help_bar::KeyHint;

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

    // Déterminer les touches à afficher.
    let mut keys = vec![
        KeyHint::new("j/k", text("naviguer", "navigate")),
        KeyHint::new("Enter", text("detail", "details")),
        KeyHint::new("b", text("branches", "branches")),
        KeyHint::new("c", text("commit", "commit")),
        KeyHint::new("s", text("stash", "stash")),
        KeyHint::new("m", text("fusion", "merge")),
        KeyHint::new("R", text("reset", "reset")),
        KeyHint::new("P", text("push", "push")),
        KeyHint::new("Ctrl+P", text("push force", "force push")),
    ];

    // Ajouter abort merge si un merge est en cours.
    if is_merging {
        keys.push(KeyHint::new("A", text("annuler fusion", "abort merge")));
    }

    // Ajouter le contexte du panneau bas.
    match bottom_left_mode {
        BottomLeftMode::Files => {
            keys.push(KeyHint::new("Tab", text("fichiers", "files")));
            keys.push(KeyHint::new("Espace", text("diff", "diff")));
            keys.push(KeyHint::new("Enter", text("plein ecran", "fullscreen")));
        }
        BottomLeftMode::Parents => keys.push(KeyHint::new("Tab", text("commit", "commit"))),
    }

    // Ajouter le raccourci pour effacer les filtres s'ils sont actifs
    if filter_active {
        keys.push(KeyHint::new(
            "Ctrl+R",
            text("effacer filtres", "clear filters"),
        ));
    }

    keys.extend([
        KeyHint::new("r", text("rafraichir", "refresh")),
        KeyHint::new("?", text("aide", "help")),
        KeyHint::new("q", text("quitter", "quit")),
    ]);

    let position = if total_commits == 0 {
        0
    } else {
        selected_index.min(total_commits - 1) + 1
    };
    let counter = format!("{position}/{total_commits}");
    crate::ui::common::help_bar::render(frame, area, &keys, Some(&counter));
}
