//! Overlay d'aide complète (touche `?`), affiche tous les raccourcis clavier.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::i18n::{text, text_owned};
use crate::state::ViewMode;
use crate::ui::common::centered_rect;
use crate::ui::keybindings;
use crate::ui::theme::current_theme;

pub struct HelpOverlayRenderContext<'a> {
    pub area: Rect,
    pub active_view: ViewMode,
    pub custom_commands: &'a [crate::config::ResolvedCustomCommand],
}

/// Rend l'overlay d'aide complet centré sur l'écran.
pub fn render(frame: &mut Frame, ctx: HelpOverlayRenderContext<'_>) {
    let HelpOverlayRenderContext {
        area,
        active_view,
        custom_commands,
    } = ctx;

    let theme = current_theme();
    // Créer une zone centrale pour le popup (70% largeur, 80% hauteur).
    let popup_area = centered_rect(70, 80, area);

    // Effacer l'arrière-plan derrière le popup.
    frame.render_widget(Clear, popup_area);

    // Construire le contenu de l'aide.
    let content = build_help_content(active_view, custom_commands);

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
fn build_help_content(
    active_view: ViewMode,
    custom_commands: &[crate::config::ResolvedCustomCommand],
) -> Vec<Line<'static>> {
    let theme = current_theme();
    let mut lines = vec![Line::from("")];

    append_global_help(&mut lines);
    lines.push(Line::from(""));
    append_view_help(&mut lines, active_view);
    if !custom_commands.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header(text(
            "Commandes personnalisees",
            "Custom commands",
        )));
        lines.push(separator());
        for command in custom_commands {
            lines.push(key_line(&command.definition.key, &command.definition.name));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        text_owned("Esc ou ? pour fermer", "Esc or ? to close"),
        Style::default()
            .fg(theme.text_secondary)
            .add_modifier(Modifier::ITALIC),
    )]));

    lines
}

fn append_global_help(lines: &mut Vec<Line<'static>>) {
    lines.push(section_header(text("Global", "Global")));
    lines.push(separator());
    lines.push(key_line(
        keybindings::global::VIEW_GRAPH,
        text("Vue Graph", "Graph view"),
    ));
    lines.push(key_line(
        keybindings::global::VIEW_STAGING,
        text("Vue Staging", "Staging view"),
    ));
    lines.push(key_line(
        keybindings::global::VIEW_BRANCHES,
        text("Vue Branches", "Branches view"),
    ));
    lines.push(key_line(
        keybindings::global::VIEW_PROJECT_TREE,
        text("Vue Arborescence", "Project tree view"),
    ));
    lines.push(key_line(
        keybindings::global::VIEW_WORKTREES,
        text("Selecteur de worktrees", "Worktree selector"),
    ));
    lines.push(key_line(
        keybindings::global::VIEW_CONFLICTS,
        text("Vue Conflits (si actifs)", "Conflicts view (if active)"),
    ));
    lines.push(key_line(keybindings::global::HELP, text("Aide", "Help")));
    lines.push(key_line(
        keybindings::global::REFRESH,
        text("Rafraichir", "Refresh"),
    ));
    lines.push(key_line(
        keybindings::global::COPY,
        text("Copier dans le presse-papiers", "Copy to clipboard"),
    ));
    lines.push(key_line_multi(
        keybindings::global::QUIT,
        text("Quitter", "Quit"),
    ));
}

fn append_view_help(lines: &mut Vec<Line<'static>>, active_view: ViewMode) {
    match active_view {
        ViewMode::Graph | ViewMode::Help => append_graph_help(lines),
        ViewMode::Staging => append_staging_help(lines),
        ViewMode::Branches => append_branches_help(lines),
        ViewMode::ProjectTree => append_project_tree_help(lines),
        ViewMode::Blame => append_blame_help(lines),
        ViewMode::Conflicts => append_conflicts_help(lines),
    }
}

fn append_project_tree_help(lines: &mut Vec<Line<'static>>) {
    lines.push(section_header(text(
        "Vue Arborescence",
        "Project Tree View",
    )));
    lines.push(separator());
    lines.push(key_line_multi(
        keybindings::navigation::DOWN,
        text("Selection suivante", "Next selection"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::UP,
        text("Selection precedente", "Previous selection"),
    ));
    lines.push(key_line(
        keybindings::project_tree::TOGGLE,
        text("Ouvrir/fermer un dossier", "Expand/collapse directory"),
    ));
    lines.push(key_line(
        keybindings::project_tree::COLLAPSE,
        text("Fermer ou remonter au parent", "Collapse or select parent"),
    ));
    lines.push(key_line(
        keybindings::project_tree::EXPAND,
        text("Ouvrir un dossier", "Expand directory"),
    ));
    lines.push(key_line(
        keybindings::project_tree::SEARCH,
        text("Recherche rapide de chemin", "Quick path search"),
    ));
    lines.push(key_line(
        keybindings::project_tree::SWITCH_PANEL,
        text(
            "Arbre → historique → fichiers → diff",
            "Tree → history → files → diff",
        ),
    ));
    lines.push(key_line(
        keybindings::project_tree::COMPARE,
        text(
            "Comparer les commits du chemin avec une branche",
            "Compare path commits with a branch",
        ),
    ));
    lines.push(key_line(
        keybindings::project_tree::CLOSE_COMPARISON,
        text("Fermer la comparaison de chemin", "Close path comparison"),
    ));
    lines.push(key_line(
        keybindings::global::COPY,
        text(
            "Copier le chemin, commit, contenu ou diff actif",
            "Copy active path, commit, file contents or diff",
        ),
    ));
    lines.push(key_line(
        keybindings::global::REFRESH,
        text("Rafraichir", "Refresh"),
    ));
    lines.push(key_line(
        keybindings::diff::EXTERNAL,
        text("Ouvrir le diff externe", "Open external diff"),
    ));
    lines.push(key_line(
        "n / N",
        text("Hunk suivant / precedent", "Next / previous hunk"),
    ));
}

fn append_graph_help(lines: &mut Vec<Line<'static>>) {
    lines.push(section_header(text("Vue Graph", "Graph View")));
    lines.push(separator());
    lines.push(key_line_multi(
        keybindings::navigation::DOWN,
        text("Commit suivant", "Next commit"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::UP,
        text("Commit precedent", "Previous commit"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::TOP,
        text("Premier commit", "First commit"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::BOTTOM,
        text("Dernier commit", "Last commit"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::PAGE_DOWN,
        text("Page suivante", "Next page"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::PAGE_UP,
        text("Page precedente", "Previous page"),
    ));
    lines.push(key_line(
        keybindings::navigation::SWITCH_PANEL,
        text("Basculer panneaux", "Switch panels"),
    ));
    lines.push(key_line("Espace", text("Ouvrir le diff", "Open diff")));
    lines.push(key_line(
        "Enter",
        text(
            "Plein ecran / action contextuelle",
            "Fullscreen / contextual action",
        ),
    ));
    lines.push(key_line(
        keybindings::git_actions::COMMIT,
        text("Nouveau commit", "New commit"),
    ));
    lines.push(key_line(
        keybindings::git_actions::STASH,
        text("Stash rapide", "Quick stash"),
    ));
    lines.push(key_line(
        keybindings::git_actions::MERGE,
        text("Merge", "Merge"),
    ));
    lines.push(key_line(
        keybindings::git_actions::PUSH,
        text("Push", "Push"),
    ));
    lines.push(key_line(
        keybindings::git_actions::FORCE_PUSH,
        text("Force push", "Force push"),
    ));
    lines.push(key_line(
        keybindings::git_actions::BLAME,
        text("Blame du fichier", "File blame"),
    ));
    lines.push(key_line(
        keybindings::git_actions::RESET,
        text("Reset", "Reset"),
    ));
    lines.push(key_line(
        keybindings::git_actions::INTERACTIVE_REBASE,
        text(
            "Rebase interactif depuis le commit",
            "Interactive rebase from commit",
        ),
    ));
    lines.push(key_line(
        keybindings::git_actions::UNDO_REFLOG,
        text("Annuler via le reflog", "Undo from reflog"),
    ));
    lines.push(key_line(
        keybindings::git_actions::CREATE_TAG,
        text("Creer un tag sur le commit", "Create tag on commit"),
    ));
    lines.push(key_line(
        keybindings::git_actions::DELETE_TAG,
        text("Supprimer le tag du commit", "Delete commit tag"),
    ));
    lines.push(key_line(
        keybindings::git_actions::COMPARE_HEAD,
        text("Comparer le commit a HEAD", "Compare commit with HEAD"),
    ));
    lines.push(key_line(
        keybindings::git_actions::BISECT_START,
        text("Demarrer un bisect", "Start bisect"),
    ));
    lines.push(key_line(
        keybindings::git_actions::REPOSITORY_INSIGHTS,
        text(
            "Diagnostic hooks, signature et sous-modules",
            "Hooks, signature and submodule diagnostics",
        ),
    ));
    lines.push(key_line(
        keybindings::git_actions::GITHUB_PR,
        text(
            "Statut de la pull request GitHub",
            "GitHub pull request status",
        ),
    ));
    lines.push(key_line(
        "[ / ] / \\",
        text(
            "Bisect : bon / mauvais / terminer",
            "Bisect: good / bad / reset",
        ),
    ));
    lines.push(key_line(
        keybindings::search::OPEN,
        text("Ouvrir la recherche", "Open search"),
    ));
    lines.push(key_line(
        keybindings::search::FILTER,
        text("Filtre avance", "Advanced filter"),
    ));
    lines.push(key_line(
        keybindings::diff::TOGGLE_VIEW,
        text("Basculer le mode diff", "Toggle diff mode"),
    ));
    lines.push(key_line(
        keybindings::diff::EXTERNAL,
        text("Ouvrir le diff externe", "Open external diff"),
    ));
    lines.push(key_line(
        "n / N",
        text("Hunk suivant / precedent", "Next / previous hunk"),
    ));
}

fn append_staging_help(lines: &mut Vec<Line<'static>>) {
    lines.push(section_header(text("Vue Staging", "Staging View")));
    lines.push(separator());
    lines.push(key_line_multi(
        keybindings::navigation::DOWN,
        text("Selection suivante", "Next selection"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::UP,
        text("Selection precedente", "Previous selection"),
    ));
    lines.push(key_line_multi(
        keybindings::staging::STAGE,
        text("Indexer le fichier", "Stage file"),
    ));
    lines.push(key_line(
        keybindings::staging::STAGE_ALL,
        text("Indexer tous les fichiers", "Stage all files"),
    ));
    lines.push(key_line_multi(
        keybindings::staging::UNSTAGE,
        text("Desindexer le fichier", "Unstage file"),
    ));
    lines.push(key_line(
        keybindings::staging::UNSTAGE_ALL,
        text("Desindexer tous les fichiers", "Unstage all files"),
    ));
    lines.push(key_line(
        keybindings::staging::DISCARD,
        text("Abandonner le fichier", "Discard file"),
    ));
    lines.push(key_line(
        keybindings::staging::DISCARD_ALL,
        text("Abandonner tous les fichiers", "Discard all files"),
    ));
    lines.push(key_line(
        keybindings::staging::STASH_FILE,
        text("Stash du fichier", "Stash file"),
    ));
    lines.push(key_line(
        keybindings::staging::STASH_ALL,
        text("Stash des non indexes", "Stash unstaged files"),
    ));
    lines.push(key_line_multi(
        keybindings::staging::SWITCH_FOCUS,
        text("Basculer le focus", "Switch focus"),
    ));
    lines.push(key_line(
        keybindings::staging::OPEN_DIFF,
        text("Ouvrir le diff", "Open diff"),
    ));
    lines.push(key_line(
        keybindings::staging::STAGE_HUNK,
        text("Indexer le hunk dans le diff", "Stage hunk in diff"),
    ));
    lines.push(key_line(
        keybindings::staging::STAGE_LINE,
        text("Indexer la ligne dans le diff", "Stage line in diff"),
    ));
    lines.push(key_line(
        keybindings::staging::COMMIT_MESSAGE,
        text("Ecrire un commit", "Write commit"),
    ));
    lines.push(key_line(
        keybindings::staging::AMEND,
        text("Amender le commit", "Amend commit"),
    ));
    lines.push(key_line(
        keybindings::git_actions::PUSH,
        text("Push", "Push"),
    ));
    lines.push(key_line(
        keybindings::git_actions::FORCE_PUSH,
        text("Force push", "Force push"),
    ));
    lines.push(key_line(
        keybindings::diff::TOGGLE_VIEW,
        text("Basculer le mode diff", "Toggle diff mode"),
    ));
    lines.push(key_line(
        keybindings::diff::EXTERNAL,
        text("Ouvrir le diff externe", "Open external diff"),
    ));
    lines.push(key_line(
        "n / N",
        text("Hunk suivant / precedent", "Next / previous hunk"),
    ));
}

fn append_branches_help(lines: &mut Vec<Line<'static>>) {
    lines.push(section_header(text("Vue Branches", "Branches View")));
    lines.push(separator());
    lines.push(key_line_multi(
        keybindings::navigation::DOWN,
        text("Selection suivante", "Next selection"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::UP,
        text("Selection precedente", "Previous selection"),
    ));
    lines.push(key_line(
        keybindings::branches::CHECKOUT,
        text("Checkout branche", "Checkout branch"),
    ));
    lines.push(key_line(
        keybindings::branches::NEW,
        text("Nouvelle branche locale", "New local branch"),
    ));
    lines.push(key_line(
        keybindings::branches::DELETE,
        text("Supprimer branche", "Delete branch"),
    ));
    lines.push(key_line(
        keybindings::branches::RENAME,
        text("Renommer branche", "Rename branch"),
    ));
    lines.push(key_line(
        keybindings::branches::MERGE,
        text("Fusionner une branche", "Merge a branch"),
    ));
    lines.push(key_line(
        keybindings::branches::REBASE,
        text("Rebase sur une branche", "Rebase onto a branch"),
    ));
    lines.push(key_line(
        keybindings::branches::TOGGLE_REMOTE,
        text("Afficher/masquer les distantes", "Toggle remotes"),
    ));
    lines.push(key_line(
        keybindings::branches::NEXT_SECTION,
        text("Section suivante", "Next section"),
    ));
    lines.push(key_line(
        keybindings::branches::PREV_SECTION,
        text("Section precedente", "Previous section"),
    ));
    lines.push(key_line(
        keybindings::branches::WORKTREE_NEW,
        text("Nouveau worktree", "New worktree"),
    ));
    lines.push(key_line(
        keybindings::branches::WORKTREE_OPEN,
        text("Ouvrir le worktree", "Open worktree"),
    ));
    lines.push(key_line(
        keybindings::branches::WORKTREE_DELETE,
        text("Supprimer worktree", "Delete worktree"),
    ));
    lines.push(key_line(
        keybindings::branches::STASH_SAVE,
        text("Sauver un stash", "Save stash"),
    ));
    lines.push(key_line(
        keybindings::branches::STASH_APPLY,
        text("Appliquer un stash", "Apply stash"),
    ));
    lines.push(key_line(
        keybindings::branches::STASH_POP,
        text("Pop un stash", "Pop stash"),
    ));
    lines.push(key_line(
        keybindings::branches::STASH_DROP,
        text("Supprimer un stash", "Drop stash"),
    ));
    lines.push(key_line(
        keybindings::git_actions::PUSH,
        text("Push", "Push"),
    ));
    lines.push(key_line(
        keybindings::git_actions::FORCE_PUSH,
        text("Force push", "Force push"),
    ));
}

fn append_blame_help(lines: &mut Vec<Line<'static>>) {
    lines.push(section_header(text("Vue Blame", "Blame View")));
    lines.push(separator());
    lines.push(key_line_multi(
        keybindings::navigation::DOWN,
        text("Ligne suivante", "Next line"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::UP,
        text("Ligne precedente", "Previous line"),
    ));
    lines.push(key_line_multi(
        keybindings::blame::CLOSE,
        text("Fermer le blame", "Close blame"),
    ));
    lines.push(key_line(
        keybindings::blame::JUMP,
        text("Aller au commit", "Jump to commit"),
    ));
}

fn append_conflicts_help(lines: &mut Vec<Line<'static>>) {
    lines.push(section_header(text("Vue Conflits", "Conflicts View")));
    lines.push(separator());
    lines.push(key_line_multi(
        keybindings::conflicts::SWITCH_PANEL,
        text("Basculer les panneaux", "Switch panels"),
    ));
    lines.push(key_line_multi(
        keybindings::conflicts::ACCEPT_OURS,
        text("Accepter ours", "Accept ours"),
    ));
    lines.push(key_line_multi(
        keybindings::conflicts::ACCEPT_THEIRS,
        text("Accepter theirs", "Accept theirs"),
    ));
    lines.push(key_line(
        keybindings::conflicts::ACCEPT_BOTH,
        text("Accepter les deux", "Accept both"),
    ));
    lines.push(key_line(
        keybindings::conflicts::MARK_RESOLVED,
        text("Marquer comme resolu", "Mark resolved"),
    ));
    lines.push(key_line(
        keybindings::conflicts::FINALIZE,
        text("Finaliser l'operation", "Finalize operation"),
    ));
    lines.push(key_line(
        keybindings::conflicts::ABORT,
        text("Annuler l'operation", "Abort operation"),
    ));
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
    let padding = 16usize.saturating_sub(Line::from(key).width());
    Line::from(vec![
        Span::styled(key.to_string(), Style::default().fg(theme.primary)),
        Span::raw(format!("{}{}", " ".repeat(padding), desc)),
    ])
}

fn key_line_multi(keys: &[&str], desc: &str) -> Line<'static> {
    let theme = current_theme();
    let keys_str = keys.join(" / ");
    let padding = 16usize.saturating_sub(Line::from(keys_str.as_str()).width());
    Line::from(vec![
        Span::styled(keys_str, Style::default().fg(theme.primary)),
        Span::raw(format!("{}{}", " ".repeat(padding), desc)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines_to_text(lines: Vec<Line<'static>>) -> String {
        lines
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content.into_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_branches_help_contains_rebase() {
        let content = lines_to_text(build_help_content(ViewMode::Branches, &[]));

        assert!(content.contains("Rebase"));
        assert!(content.contains("e"));
        assert!(!content.contains("Indexer le fichier"));
    }

    #[test]
    fn test_staging_help_contains_discard_and_stage() {
        let content = lines_to_text(build_help_content(ViewMode::Staging, &[]));

        assert!(content.contains("Indexer le fichier") || content.contains("Stage file"));
        assert!(content.contains("Abandonner le fichier") || content.contains("Discard file"));
        assert!(!content.contains("Nouvelle branche locale"));
    }
}
