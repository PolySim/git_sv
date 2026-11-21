//! Overlay d'aide complète (touche `?`), affiche tous les raccourcis clavier.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::config::RuntimeCustomization;
use crate::i18n::text;
use crate::state::{help::HelpState, ViewMode};
use crate::ui::common::centered_rect_fixed;
use crate::ui::keybindings;
use crate::ui::theme::current_theme;

pub struct HelpOverlayRenderContext<'a> {
    pub area: Rect,
    pub active_view: ViewMode,
    pub customization: &'a RuntimeCustomization,
    pub state: &'a mut HelpState,
}

pub(crate) fn popup_area(area: Rect) -> Rect {
    centered_rect_fixed(76, 22, area)
}

/// Rend l'aide avec un en-tête et un pied hors de la zone défilante.
pub fn render(frame: &mut Frame, ctx: HelpOverlayRenderContext<'_>) {
    let popup = popup_area(ctx.area);
    let theme = current_theme();
    let block = Block::default().style(Style::default().bg(theme.background).fg(theme.text_normal));
    let block = if popup.height >= 5 && popup.width >= 12 {
        block
            .title(text(" Aide ", " Help "))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.primary))
    } else {
        block
    };
    let inner = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    let header_height = u16::from(inner.height >= 3);
    let footer_height = u16::from(inner.height > 0);
    let body = Rect::new(
        inner.x,
        inner.y + header_height,
        inner.width,
        inner.height - header_height - footer_height,
    );
    let content = build_help_content(ctx.active_view, ctx.customization, &ctx.state.query);
    let rows = wrap_rows(content, usize::from(body.width));
    ctx.state.set_viewport(rows.len(), usize::from(body.height));
    frame.render_widget(
        Paragraph::new(
            rows.into_iter()
                .skip(ctx.state.offset)
                .take(usize::from(body.height))
                .collect::<Vec<_>>(),
        ),
        body,
    );

    if header_height > 0 {
        render_query(
            frame,
            Rect::new(inner.x, inner.y, inner.width, 1),
            ctx.state,
        );
    }
    if footer_height > 0 {
        let footer = Rect::new(inner.x, inner.bottom() - 1, inner.width, 1);
        render_footer(frame, footer, ctx.state);
    }
}

fn render_query(frame: &mut Frame, area: Rect, state: &HelpState) {
    let theme = current_theme();
    let long = text("/ Rechercher : ", "/ Search: ");
    let label = if usize::from(area.width) > Span::raw(long).width() + 8 {
        long
    } else {
        "/ "
    };
    let prefix = Span::styled(label, Style::default().fg(theme.primary));
    let prefix = if state.searching {
        prefix.add_modifier(Modifier::REVERSED)
    } else {
        prefix
    };
    let query = Line::raw(crate::utils::terminal_text(&state.query).into_owned());
    let prefix_width = prefix.width();
    let available = usize::from(area.width).saturating_sub(prefix_width);
    let left = query
        .width()
        .saturating_sub(available.saturating_sub(usize::from(state.searching)));
    if state.searching && available > 0 {
        frame.set_cursor_position((
            area.x + (prefix_width + query.width() - left) as u16,
            area.y,
        ));
    }
    let mut line = Line::from(prefix);
    line.spans
        .extend(crate::ui::text_edit::clip_line(query, left, available).spans);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &HelpState) {
    let (full, compact, short, minimal) = if state.searching {
        (
            text(
                "Entrée/Esc : parcourir • Ctrl+U : effacer • ↑↓ PgUp/PgDn",
                "Enter/Esc: browse • Ctrl+U: clear • ↑↓ PgUp/PgDn",
            ),
            text(
                "Entrée/Esc : parcourir • ↑↓ PgUp/PgDn",
                "Enter/Esc: browse • ↑↓ PgUp/PgDn",
            ),
            text("Entrée/Esc • ↑↓", "Enter/Esc • ↑↓"),
            "Esc",
        )
    } else {
        (
            text(
                "Esc/? : fermer • / : rechercher • ↑↓ PgUp/PgDn",
                "Esc/?: close • /: search • ↑↓ PgUp/PgDn",
            ),
            text(
                "Esc/? : fermer • / • ↑↓ PgUp/PgDn",
                "Esc/?: close • / • ↑↓ PgUp/PgDn",
            ),
            "Esc/? • / • ↑↓",
            "Esc/? /",
        )
    };
    let label = [full, compact, short, minimal]
        .into_iter()
        .find(|label| Span::raw(*label).width() <= usize::from(area.width))
        .unwrap_or(minimal);
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(current_theme().text_secondary)),
        area,
    );
}

#[derive(Debug)]
enum HelpRow {
    Section(&'static str),
    Entry { key: String, label: String },
}

fn build_help_content(
    active_view: ViewMode,
    customization: &RuntimeCustomization,
    query: &str,
) -> Vec<Line<'static>> {
    let mut catalog = Vec::new();
    append_view_help(&mut catalog, active_view);
    append_global_help(&mut catalog, active_view);
    catalog.push(section_header(text(
        "Commandes personnalisées",
        "Custom commands",
    )));
    for command in &customization.custom_commands {
        catalog.push(key_line(&command.definition.key, &command.definition.name));
    }
    catalog.push(section_header(text(
        "Raccourcis configurés",
        "Configured shortcuts",
    )));
    for binding in &customization.keybindings {
        catalog.push(key_line(&binding.key, &binding.action));
    }
    matching_lines(catalog, query)
}

fn matching_lines(catalog: Vec<HelpRow>, query: &str) -> Vec<Line<'static>> {
    let theme = current_theme();
    let query = query.trim().to_lowercase();
    let mut lines = Vec::new();
    let mut pending_section = None;
    for row in catalog {
        match row {
            HelpRow::Section(title) => pending_section = Some(title),
            HelpRow::Entry { key, label }
                if key.to_lowercase().contains(&query) || label.to_lowercase().contains(&query) =>
            {
                if let Some(title) = pending_section.take() {
                    if !lines.is_empty() {
                        lines.push(Line::default());
                    }
                    lines.push(Line::styled(
                        title,
                        Style::default().fg(theme.warning).bold(),
                    ));
                }
                let key = crate::utils::terminal_text(&key).into_owned();
                let label = crate::utils::terminal_text(&label).into_owned();
                lines.push(Line::from(vec![
                    Span::styled(key, Style::default().fg(theme.primary)),
                    Span::raw(format!("  {label}")),
                ]));
            }
            HelpRow::Entry { .. } => {}
        }
    }
    if lines.is_empty() {
        lines.push(Line::styled(
            text("Aucun résultat", "No matches"),
            Style::default().fg(theme.text_secondary),
        ));
    }
    lines
}

/// Produit les lignes visuelles avant de calculer les limites de défilement.
fn wrap_rows(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for mut line in lines {
        // Aligner les libellés si la largeur le permet, sans gaspiller les petits écrans.
        if width >= 52 && line.spans.len() == 2 {
            let padding = 12usize.saturating_sub(line.spans[0].width());
            line.spans.insert(1, Span::raw(" ".repeat(padding)));
        }
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut used = 0;
        for grapheme in line.styled_graphemes(Style::default()) {
            let cells = Span::raw(grapheme.symbol).width();
            if used > 0 && used + cells > width {
                rows.push(Line::from(std::mem::take(&mut spans)));
                used = 0;
            }
            if cells > width {
                continue;
            }
            if let Some(span) = spans.last_mut().filter(|span| span.style == grapheme.style) {
                span.content.to_mut().push_str(grapheme.symbol);
            } else {
                spans.push(Span::styled(grapheme.symbol.to_owned(), grapheme.style));
            }
            used += cells;
        }
        rows.push(Line::from(spans));
    }
    rows
}

fn append_global_help(lines: &mut Vec<HelpRow>, view: ViewMode) {
    lines.push(section_header(text("Global", "Global")));
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
    if matches!(view, ViewMode::Graph | ViewMode::Staging) {
        lines.push(key_line(
            keybindings::global::REFRESH,
            text("Rafraîchir", "Refresh"),
        ));
    }
    if view != ViewMode::Conflicts {
        lines.push(key_line(
            keybindings::global::COPY,
            text("Copier dans le presse-papiers", "Copy to clipboard"),
        ));
    }
    if matches!(view, ViewMode::Blame | ViewMode::Conflicts) {
        lines.push(key_line("Ctrl+C", text("Quitter", "Quit")));
    } else {
        lines.push(key_line_multi(
            keybindings::global::QUIT,
            text("Quitter", "Quit"),
        ));
    }
}

fn append_view_help(lines: &mut Vec<HelpRow>, active_view: ViewMode) {
    match active_view {
        ViewMode::Graph | ViewMode::Help => append_graph_help(lines),
        ViewMode::Staging => append_staging_help(lines),
        ViewMode::Branches => append_branches_help(lines),
        ViewMode::ProjectTree => append_project_tree_help(lines),
        ViewMode::Blame => append_blame_help(lines),
        ViewMode::Conflicts => append_conflicts_help(lines),
        ViewMode::Comparison => {}
    }
}

fn append_project_tree_help(lines: &mut Vec<HelpRow>) {
    lines.push(section_header(text(
        "Vue Arborescence",
        "Project Tree View",
    )));
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

fn append_graph_help(lines: &mut Vec<HelpRow>) {
    lines.push(section_header(text("Vue Graph", "Graph View")));
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
    lines.push(key_line(
        text("Espace", "Space"),
        text("Ouvrir le diff", "Open diff"),
    ));
    lines.push(key_line(
        "Enter",
        text(
            "Plein ecran / action contextuelle",
            "Fullscreen / contextual action",
        ),
    ));
    lines.push(key_line(
        keybindings::git_actions::LOAD_MORE,
        text("Charger plus d’historique", "Load more history"),
    ));
    lines.push(section_header(text("Opérations Git", "Git operations")));
    lines.push(key_line(keybindings::git_actions::FETCH, "Fetch"));
    lines.push(key_line(keybindings::git_actions::PULL, "Pull"));
    lines.push(key_line(
        keybindings::git_actions::CHERRY_PICK,
        "Cherry-pick",
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
        "a",
        text("Marquer ou remplacer A", "Mark or replace A"),
    ));
    lines.push(key_line(
        "D",
        text(
            "Comparer A → B (commit sélectionné)",
            "Compare A → B (selected commit)",
        ),
    ));
    lines.push(key_line("Ctrl+A", text("Effacer A", "Clear A")));
    lines.push(key_line(
        keybindings::git_actions::BISECT_START,
        text("Démarrer un bisect", "Start bisect"),
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
    lines.push(section_header(text("Recherche et diff", "Search and diff")));
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

fn append_staging_help(lines: &mut Vec<HelpRow>) {
    lines.push(section_header(text("Vue Staging", "Staging View")));
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
        keybindings::staging::UNSTAGE_HUNK,
        text("Désindexer le hunk dans le diff", "Unstage hunk in diff"),
    ));
    lines.push(key_line(
        keybindings::staging::UNSTAGE_LINE,
        text("Désindexer la ligne dans le diff", "Unstage line in diff"),
    ));
    lines.push(key_line(
        keybindings::staging::COMMIT_MESSAGE,
        text("Écrire un commit", "Write commit"),
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

fn append_branches_help(lines: &mut Vec<HelpRow>) {
    lines.push(section_header(text("Vue Branches", "Branches View")));
    lines.push(key_line(
        keybindings::branches::FIND,
        text(
            "Trouver une branche (→ sélectionne sans checkout)",
            "Find a branch (→ selects without checkout)",
        ),
    ));
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
    lines.push(section_header(text("Worktrees", "Worktrees")));
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
    lines.push(section_header(text("Stashes", "Stashes")));
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
    lines.push(section_header(text("Toutes les sections", "All sections")));
    lines.push(key_line(
        keybindings::git_actions::PUSH,
        text("Push", "Push"),
    ));
}

fn append_blame_help(lines: &mut Vec<HelpRow>) {
    lines.push(section_header(text("Vue Blame", "Blame View")));
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

fn append_conflicts_help(lines: &mut Vec<HelpRow>) {
    lines.push(section_header(text("Vue Conflits", "Conflicts View")));
    lines.push(key_line_multi(
        keybindings::navigation::DOWN,
        text("Élément suivant", "Next item"),
    ));
    lines.push(key_line_multi(
        keybindings::navigation::UP,
        text("Élément précédent", "Previous item"),
    ));
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
        "r / Enter",
        text(
            "Marquer comme résolu (selon le panneau)",
            "Mark resolved (depending on panel)",
        ),
    ));
    lines.push(key_line(
        keybindings::conflicts::FINALIZE,
        text("Finaliser l'operation", "Finalize operation"),
    ));
    lines.push(key_line(
        keybindings::conflicts::ABORT,
        text("Annuler l'opération", "Abort operation"),
    ));
    lines.push(key_line(
        "F / B / L",
        text("Mode fichier / bloc / ligne", "File / block / line mode"),
    ));
    lines.push(key_line(
        text("Espace", "Space"),
        text(
            "Choisir un bloc ou basculer une ligne",
            "Choose a block or toggle a line",
        ),
    ));
    lines.push(key_line(
        "Enter",
        text(
            "Valider le fichier en mode fichier",
            "Confirm the file in file mode",
        ),
    ));
    lines.push(key_line_multi(
        keybindings::conflicts::EDIT,
        text("Éditer le résultat", "Edit the result"),
    ));
    lines.push(key_line(
        "Ctrl+S / Ctrl+Enter",
        text(
            "Enregistrer le résultat et le marquer résolu",
            "Save the result and mark it resolved",
        ),
    ));
    lines.push(key_line("Esc", text("Annuler l'édition", "Cancel editing")));
    lines.push(key_line(
        "↑ / ↓ / ← / →",
        text(
            "Déplacer le curseur en édition",
            "Move the cursor while editing",
        ),
    ));
    lines.push(key_line(
        "Backspace / Delete",
        text(
            "Supprimer un caractère en édition",
            "Delete a character while editing",
        ),
    ));
    lines.push(key_line(
        "Enter",
        text(
            "Insérer une nouvelle ligne en édition",
            "Insert a new line while editing",
        ),
    ));
    lines.push(key_line(
        "q / Esc",
        text(
            "Revenir au staging sans annuler l'opération",
            "Return to staging without aborting the operation",
        ),
    ));
}

fn section_header(title: &'static str) -> HelpRow {
    HelpRow::Section(title)
}

fn key_line(key: &str, label: &str) -> HelpRow {
    HelpRow::Entry {
        key: key.to_owned(),
        label: label.to_owned(),
    }
}

fn key_line_multi(keys: &[&str], label: &str) -> HelpRow {
    key_line(&keys.join(" / "), label)
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
    fn help_filters_the_active_locale_by_label_or_key() {
        use crate::i18n::{with_language, Language};

        for (language, label, other_label, header) in [
            (Language::En, "EDIT THE RESULT", "éditer", "Conflicts View"),
            (Language::Fr, "ÉDITER", "edit the result", "Vue Conflits"),
        ] {
            with_language(language, || {
                let content = |query| {
                    lines_to_text(build_help_content(
                        ViewMode::Conflicts,
                        &RuntimeCustomization::default(),
                        query,
                    ))
                };
                assert!(content("").starts_with(header));
                let by_label = content(label);
                assert!(by_label.starts_with(header));
                assert!(by_label.contains("i / e"));
                assert!(!by_label.contains("Global"));
                assert!(content("cTrL+s").contains("Ctrl+S"));
                assert_eq!(content(other_label), text("Aucun résultat", "No matches"));
            });
        }
    }

    #[test]
    fn help_searches_custom_keys_and_names_without_disclosing_shell_payloads() {
        use crate::config::{AppConfig, CustomCommandConfig};

        let config = AppConfig {
            keybindings: [("global.help".into(), "alt+h".into())].into(),
            custom_commands: vec![CustomCommandConfig {
                name: "Résumé local".into(),
                key: "alt+w".into(),
                command: "secret payload".into(),
                confirm: false,
                pause: false,
            }],
            ..AppConfig::default()
        }
        .runtime_customization();
        let content = |query| lines_to_text(build_help_content(ViewMode::Graph, &config, query));
        assert!(content("RÉSUMÉ").contains("alt+w"));
        assert!(content("ALT+W").contains("Résumé local"));
        assert!(content("ALT+H").contains("global.help"));
        assert!(!content("").contains("secret payload"));
    }

    #[test]
    fn help_renders_fixed_footer_and_bounded_pages_at_normal_and_small_sizes() {
        use crate::i18n::{with_language, Language};
        use crate::state::action::HelpAction;
        use ratatui::{backend::TestBackend, Terminal};

        with_language(Language::En, || {
            for (width, height) in [(80, 24), (40, 10), (20, 6), (12, 4)] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                let mut state = HelpState::default();
                let popup = popup_area(Rect::new(0, 0, width, height));
                let draw = |terminal: &mut Terminal<TestBackend>, state: &mut HelpState| {
                    terminal
                        .draw(|frame| {
                            render(
                                frame,
                                HelpOverlayRenderContext {
                                    area: frame.area(),
                                    active_view: ViewMode::Graph,
                                    customization: &RuntimeCustomization::default(),
                                    state,
                                },
                            )
                        })
                        .unwrap();
                };
                let row = |terminal: &Terminal<TestBackend>, y| {
                    (popup.x..popup.right())
                        .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                        .collect::<String>()
                };
                let bordered = height >= 5;
                let footer = popup.bottom() - if bordered { 2 } else { 1 };
                let body = popup.y + if bordered { 2 } else { 1 };
                draw(&mut terminal, &mut state);
                let footer_before = row(&terminal, footer);
                assert!(footer_before.contains("Esc/?"));
                assert!(row(&terminal, body).contains("Graph View"));
                state.apply(HelpAction::PageDown);
                assert_eq!(state.offset, usize::from(footer - body));
                draw(&mut terminal, &mut state);
                assert!(!row(&terminal, body).contains("Graph View"));
                assert_eq!(row(&terminal, footer), footer_before);
                state.apply(HelpAction::PageUp);
                state.apply(HelpAction::Up);
                assert_eq!(state.offset, 0);
                state.apply(HelpAction::Bottom);
                let last = state.offset;
                state.apply(HelpAction::PageDown);
                state.apply(HelpAction::Down);
                assert_eq!(state.offset, last);
                state.apply(HelpAction::StartSearch);
                state.apply(HelpAction::Insert("not a shortcut".into()));
                draw(&mut terminal, &mut state);
                assert!(row(&terminal, body).contains("No matches"));
                state.apply(HelpAction::Bottom);
                assert_eq!(state.offset, 0);
            }
        });
    }

    #[test]
    fn branches_help_describes_the_temporary_finder_in_both_languages() {
        use crate::i18n::{with_language, Language};

        for (language, query, expected) in [
            (
                Language::Fr,
                "trouver une branche",
                "/  Trouver une branche (→ sélectionne sans checkout)",
            ),
            (
                Language::En,
                "find a branch",
                "/  Find a branch (→ selects without checkout)",
            ),
        ] {
            with_language(language, || {
                let content = lines_to_text(build_help_content(
                    ViewMode::Branches,
                    &RuntimeCustomization::default(),
                    query,
                ));
                assert!(content.contains(expected));
            });
        }
    }

    #[test]
    fn test_branches_help_contains_rebase() {
        let content = lines_to_text(build_help_content(
            ViewMode::Branches,
            &RuntimeCustomization::default(),
            "",
        ));

        assert!(content.contains("Rebase"));
        assert!(content.contains("e"));
        assert!(!content.contains("Indexer le fichier"));
    }

    #[test]
    fn test_staging_help_contains_discard_and_stage() {
        let content = lines_to_text(build_help_content(
            ViewMode::Staging,
            &RuntimeCustomization::default(),
            "",
        ));

        assert!(content.contains("Indexer le fichier") || content.contains("Stage file"));
        assert!(content.contains("Abandonner le fichier") || content.contains("Discard file"));
        assert!(!content.contains("Nouvelle branche locale"));
    }
}
