//! Rendu de l'arborescence courante et de son historique.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::git::diff::DiffStatus;
use crate::git::project_tree::PathHistorySide;
use crate::i18n::{text, text_owned};
use crate::state::{selection_range, AppState, ProjectEntryKind, ProjectTreeFocus, ViewMode};
use crate::ui::common::help_bar::KeyHint;
use crate::ui::project_tree_layout::build_project_tree_layout;
use crate::ui::theme::{current_theme, Theme};
use crate::utils::time::format_relative_time;

pub fn render(frame: &mut Frame, state: &mut AppState, unresolved_conflicts: usize) {
    let search_active = state.project_tree_state.search.is_active;
    let layout = build_project_tree_layout(frame.area(), search_active);
    let tree_height = layout.tree_panel.height.saturating_sub(2) as usize;
    let history_height = layout.history_panel.height.saturating_sub(2) as usize;
    let files_height = layout.changed_files_panel.height.saturating_sub(2) as usize;
    state
        .project_tree_state
        .entries
        .set_visible_height(tree_height.max(1));
    state
        .project_tree_state
        .search
        .results
        .set_visible_height(tree_height.max(1));
    state
        .project_tree_state
        .history
        .set_visible_height(history_height.max(1));
    state
        .project_tree_state
        .changed_files
        .set_visible_height(files_height.max(1));

    super::status_bar::render(
        frame,
        super::status_bar::StatusBarRenderContext {
            current_branch: state.current_branch.as_deref(),
            status_entries: &state.status_entries,
            flash_message: state.current_flash_message(),
            filter: &state.filters.graph_filter,
            is_merging: state.ui.is_merging,
            area: layout.status_bar,
        },
    );
    super::nav_bar::render(
        frame,
        super::nav_bar::NavBarRenderContext {
            current_view: ViewMode::ProjectTree,
            area: layout.nav_bar,
            unresolved_conflicts,
        },
    );

    if let Some(area) = layout.search_bar {
        render_search(frame, state, area);
    }
    render_tree(frame, state, layout.tree_panel);
    render_history(frame, state, layout.history_panel);
    render_changed_files(frame, state, layout.changed_files_panel);
    render_diff(frame, state, layout.diff_panel);

    let mut hints = vec![
        KeyHint::new("j/k", text("naviguer", "navigate")),
        KeyHint::new("C", text("comparer", "compare")),
    ];
    if state.project_tree_state.comparison.is_some() {
        hints.push(KeyHint::new(
            "Esc",
            text("fermer comparaison", "close comparison"),
        ));
    }
    hints.extend([
        KeyHint::new("Tab", text("changer panneau", "switch panel")),
        KeyHint::new("←/→", text("fermer/ouvrir", "collapse/expand")),
        KeyHint::new("/", text("rechercher", "search")),
        KeyHint::new("y", text("copier", "copy")),
        KeyHint::new("?", text("aide", "help")),
    ]);
    let trailing = state
        .project_tree_state
        .selected_entry()
        .map(|entry| entry.path.as_str());
    super::common::help_bar::render(frame, layout.help_bar, &hints, trailing);
}

fn render_tree(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let theme = current_theme();
    let focused = state.project_tree_state.focus == ProjectTreeFocus::Tree;
    let search_active = state.project_tree_state.search.is_active;
    let selection = if search_active {
        &state.project_tree_state.search.results
    } else {
        &state.project_tree_state.entries
    };
    let items: Vec<_> = selection
        .visible_items()
        .map(|(_, entry)| {
            let marker = match (entry.kind, entry.expanded, search_active) {
                (ProjectEntryKind::Directory, _, true) => "▸ ",
                (ProjectEntryKind::Directory, true, false) => "▾ ",
                (ProjectEntryKind::Directory, false, false) => "▸ ",
                (ProjectEntryKind::File, _, _) => "  ",
            };
            let style = if entry.kind == ProjectEntryKind::Directory {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_normal)
            };
            let label = if search_active {
                entry.path.clone()
            } else {
                entry.name.clone()
            };
            ListItem::new(Line::from(vec![
                Span::raw(if search_active {
                    String::new()
                } else {
                    "  ".repeat(entry.depth)
                }),
                Span::styled(marker, style),
                Span::styled(label, style),
            ]))
        })
        .collect();
    let title: String = if search_active {
        text_owned(
            format!("▶ Résultats ({}) ", selection.len()),
            format!("▶ Results ({}) ", selection.len()),
        )
    } else if focused {
        text("▶ Arborescence ", "▶ Project tree ").to_string()
    } else {
        text(" Arborescence ", " Project tree ").to_string()
    };
    let list = List::new(if items.is_empty() {
        vec![ListItem::new(text("  Aucun fichier", "  No files"))]
    } else {
        items
    })
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if focused {
                theme.border_active
            } else {
                theme.border_inactive
            })),
    )
    .highlight_style(
        Style::default()
            .bg(theme.selection_bg)
            .fg(theme.selection_fg)
            .add_modifier(Modifier::BOLD),
    );
    let selected =
        (!selection.is_empty()).then_some(selection.selected_index() - selection.scroll_offset());
    let mut list_state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_search(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let theme = current_theme();
    let search = &state.project_tree_state.search;
    let cursor = search.cursor.min(search.query.chars().count());
    let selection = selection_range(cursor, search.selection_anchor);
    let mut spans = vec![Span::styled(
        "/ ",
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    )];
    for (index, character) in search.query.chars().enumerate() {
        let style = if index == cursor {
            Style::default().bg(theme.primary).fg(theme.background)
        } else if selection
            .as_ref()
            .is_some_and(|range| range.contains(&index))
        {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
        } else {
            Style::default().fg(theme.text_normal)
        };
        spans.push(Span::styled(character.to_string(), style));
    }
    if cursor == search.query.chars().count() {
        spans.push(Span::styled(" ", Style::default().bg(theme.primary)));
    }
    spans.push(Span::styled(
        text_owned(
            format!(
                "  {} résultat(s) — Entrée pour révéler",
                search.results.len()
            ),
            format!("  {} result(s) — Enter to reveal", search.results.len()),
        ),
        Style::default().fg(theme.text_secondary),
    ));
    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(
            Block::default()
                .title(text(" Recherche rapide ", " Quick search "))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        ),
        area,
    );
}

fn render_history(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let theme = current_theme();
    let focused = state.project_tree_state.focus == ProjectTreeFocus::History;
    let items: Vec<_> = state
        .project_tree_state
        .history
        .visible_items()
        .map(|(_, commit)| {
            let hash = commit.oid.to_string();
            let (side_marker, side_style) = match state.project_tree_state.history_side(commit.oid)
            {
                Some(PathHistorySide::Current) => ("+ ", Style::default().fg(theme.success)),
                Some(PathHistorySide::Target) => ("- ", Style::default().fg(theme.error)),
                None => ("  ", Style::default().fg(theme.text_secondary)),
            };
            ListItem::new(Line::from(vec![
                Span::styled(side_marker, side_style.add_modifier(Modifier::BOLD)),
                Span::styled(
                    hash[..7].to_string(),
                    Style::default().fg(theme.commit_hash),
                ),
                Span::raw("  "),
                Span::styled(
                    format_relative_time(commit.timestamp),
                    Style::default().fg(theme.text_secondary),
                ),
                Span::raw("  "),
                Span::styled(commit.author.clone(), Style::default().fg(theme.secondary)),
                Span::raw("  "),
                Span::raw(commit.message.clone()),
            ]))
        })
        .collect();
    let selected_path = state
        .project_tree_state
        .selected_entry()
        .map(|entry| entry.path.as_str())
        .unwrap_or(text("aucune selection", "no selection"));
    let title = if let Some(comparison) = state.project_tree_state.comparison.as_ref() {
        let counts = match (comparison.ahead, comparison.behind) {
            (Some(ahead), Some(behind)) => format!("+{ahead} / -{behind}"),
            _ => "…".to_string(),
        };
        text_owned(
            format!(
                "{}Historique · {} ↔ {} · {} — {} ",
                if focused { "▶ " } else { " " },
                comparison.base_branch,
                comparison.target_branch,
                counts,
                selected_path
            ),
            format!(
                "{}History · {} ↔ {} · {} — {} ",
                if focused { "▶ " } else { " " },
                comparison.base_branch,
                comparison.target_branch,
                counts,
                selected_path
            ),
        )
    } else if focused {
        text_owned(
            format!("▶ Historique — {} ", selected_path),
            format!("▶ History — {} ", selected_path),
        )
    } else {
        text_owned(
            format!(" Historique — {} ", selected_path),
            format!(" History — {} ", selected_path),
        )
    };
    let list = List::new(if items.is_empty() {
        let empty_message = if state.project_tree_state.history_loaded
            && state.project_tree_state.comparison.is_some()
        {
            text(
                "  Aucun commit divergent pour ce chemin",
                "  No divergent commit for this path",
            )
        } else if state.project_tree_state.history_loaded {
            text(
                "  Aucun commit pour ce chemin",
                "  No commits for this path",
            )
        } else {
            text(
                "  Tab ou clic pour charger l'historique",
                "  Tab or click to load history",
            )
        };
        vec![ListItem::new(empty_message)]
    } else {
        items
    })
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if focused {
                theme.border_active
            } else {
                theme.border_inactive
            })),
    )
    .highlight_style(
        Style::default()
            .bg(theme.selection_bg)
            .fg(theme.selection_fg)
            .add_modifier(Modifier::BOLD),
    );
    let selected = (!state.project_tree_state.history.is_empty()).then_some(
        state.project_tree_state.history.selected_index()
            - state.project_tree_state.history.scroll_offset(),
    );
    let mut list_state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_changed_files(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let theme = current_theme();
    let focused = state.project_tree_state.focus == ProjectTreeFocus::ChangedFiles;
    let separator_index = state.project_tree_state.changed_files_separator_index();
    let mut items = Vec::new();
    for (index, file) in state.project_tree_state.changed_files.visible_items() {
        if separator_index == Some(index) {
            items.push(ListItem::new(Line::from(Span::styled(
                text(" ── Autres fichiers ──", " ── Other files ──"),
                Style::default().fg(theme.text_secondary),
            ))));
        }
        items.push({
            let file_line = Line::from(vec![
                Span::styled(
                    format!(" {} ", file.status.display_char()),
                    Style::default().fg(diff_status_color(&file.status, theme)),
                ),
                Span::styled(
                    format!("+{} ", file.additions),
                    Style::default().fg(theme.success),
                ),
                Span::styled(
                    format!("-{} ", file.deletions),
                    Style::default().fg(theme.error),
                ),
                Span::styled(file.path.clone(), Style::default().fg(theme.text_normal)),
            ]);
            ListItem::new(file_line)
        });
    }
    let title = if focused {
        text("▶ Fichiers touchés ", "▶ Changed files ")
    } else {
        text(" Fichiers touchés ", " Changed files ")
    };
    let list = List::new(if items.is_empty() {
        let empty_message = if state.project_tree_state.commit_details_loaded {
            text("  Aucun fichier", "  No files")
        } else {
            text(
                "  Tab ou clic pour charger les fichiers",
                "  Tab or click to load files",
            )
        };
        vec![ListItem::new(empty_message)]
    } else {
        items
    })
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if focused {
                theme.border_active
            } else {
                theme.border_inactive
            })),
    )
    .highlight_style(
        Style::default()
            .bg(theme.selection_bg)
            .fg(theme.selection_fg)
            .add_modifier(Modifier::BOLD),
    );
    let selection = &state.project_tree_state.changed_files;
    let selected = (!selection.is_empty()).then(|| {
        let selected_index = selection.selected_index();
        let mut visual_index = selected_index - selection.scroll_offset();
        if separator_index.is_some_and(|separator| {
            separator >= selection.scroll_offset() && selected_index >= separator
        }) {
            visual_index += 1;
        }
        visual_index
    });
    let mut list_state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_diff(frame: &mut Frame, state: &mut AppState, area: ratatui::layout::Rect) {
    let project = &state.project_tree_state;
    if !project.diff_loaded {
        let focused = project.focus == ProjectTreeFocus::Diff;
        let theme = current_theme();
        frame.render_widget(
            Paragraph::new(text(
                "Tab ou clic pour charger le diff",
                "Tab or click to load diff",
            ))
            .block(
                Block::default()
                    .title(if focused { "▶ Diff " } else { " Diff " })
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if focused {
                        theme.border_active
                    } else {
                        theme.border_inactive
                    })),
            ),
            area,
        );
        state.project_tree_state.diff_total_lines = 1;
        return;
    }

    let total_lines = super::diff_view::render(
        frame,
        super::diff_view::DiffRenderContext {
            diff: project.selected_diff.as_deref(),
            scroll_offset: project.diff_scroll_offset,
            horizontal_offset: project.diff_horizontal_offset,
            area,
            is_focused: project.focus == ProjectTreeFocus::Diff,
            view_mode: project.diff_view_mode,
            is_fullscreen: false,
            image_state: &mut state.image_preview,
        },
    );
    state.project_tree_state.diff_total_lines = total_lines;
}

fn diff_status_color(status: &DiffStatus, theme: &Theme) -> ratatui::style::Color {
    match status {
        DiffStatus::Added => theme.success,
        DiffStatus::Modified => theme.warning,
        DiffStatus::Deleted => theme.error,
        DiffStatus::Renamed => theme.primary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff::{DiffFile, DiffStatus};
    use crate::git::repo::GitRepo;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn view_renders_tree_and_selected_path_history() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "src/main.rs", "fn main() {}", "initial tree");
        let git_repo = GitRepo::open(temp.path().to_string_lossy().as_ref()).unwrap();
        let mut state = AppState::new(git_repo, temp.path().display().to_string()).unwrap();
        state.view_mode = ViewMode::ProjectTree;
        state.refresh_project_tree();
        state.refresh_selected_path_history();
        state.refresh_selected_history_commit_details();
        state.refresh_selected_history_file_diff();

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| crate::ui::render(frame, &mut state))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let output: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
        assert!(output.contains("src"));
        assert!(output.contains("initial tree"));
        assert!(output.contains("Fichiers") || output.contains("Changed files"));
        assert!(output.contains("main.rs"));
        assert!(output.contains("fn main"));
    }

    #[test]
    fn view_explains_that_project_history_is_loaded_on_demand() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "src/main.rs", "fn main() {}", "initial tree");
        let git_repo = GitRepo::open(temp.path().to_string_lossy().as_ref()).unwrap();
        let mut state = AppState::new(git_repo, temp.path().display().to_string()).unwrap();
        state.view_mode = ViewMode::ProjectTree;
        state.refresh_project_tree();

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| crate::ui::render(frame, &mut state))
            .unwrap();

        let output: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(output.contains("Tab"));
        assert!(!output.contains("initial tree"));
    }

    #[test]
    fn view_renders_compared_path_history_with_branch_sides() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "shared.txt", "base", "common path");
        let common = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &common, false).unwrap();
        drop(common);
        commit_file(&repo, "shared.txt", "main", "main path");
        crate::git::branch::checkout_branch(&repo, "feature").unwrap();
        commit_file(&repo, "shared.txt", "feature", "feature path");
        crate::git::branch::checkout_branch(&repo, "main").unwrap();

        let git_repo = GitRepo::open(temp.path().to_string_lossy().as_ref()).unwrap();
        let mut state = AppState::new(git_repo, temp.path().display().to_string()).unwrap();
        state.view_mode = ViewMode::ProjectTree;
        state.refresh_project_tree();
        let comparison = state
            .repo
            .compare_path_history("shared.txt", false, "feature", 100)
            .unwrap();
        state
            .project_tree_state
            .start_comparison("main".to_string(), "feature".to_string());
        state
            .project_tree_state
            .set_compared_path_history(comparison);
        state.project_tree_state.focus = ProjectTreeFocus::History;

        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| crate::ui::render(frame, &mut state))
            .unwrap();

        let output: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(output.contains("main ↔ feature"));
        assert!(output.contains("+1 / -1"));
        assert!(output.contains("+ "));
        assert!(output.contains("- "));
        assert!(output.contains("main path"));
        assert!(output.contains("feature path"));
    }

    #[test]
    fn view_renders_inline_quick_search_results() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "src/project_tree.rs", "tree", "add tree");
        let git_repo = GitRepo::open(temp.path().to_string_lossy().as_ref()).unwrap();
        let mut state = AppState::new(git_repo, temp.path().display().to_string()).unwrap();
        state.view_mode = ViewMode::ProjectTree;
        state.refresh_project_tree();
        state.project_tree_state.open_search();
        state.project_tree_state.search.query = "projet_tree".to_string();
        state.project_tree_state.update_search_results();

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| crate::ui::render(frame, &mut state))
            .unwrap();

        let output: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(output.contains("Recherche rapide") || output.contains("Quick search"));
        assert!(output.contains("src/project_tree.rs"));
    }

    #[test]
    fn view_separates_files_outside_selected_directory() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "src/main.rs", "fn main() {}", "initial tree");
        let git_repo = GitRepo::open(temp.path().to_string_lossy().as_ref()).unwrap();
        let mut state = AppState::new(git_repo, temp.path().display().to_string()).unwrap();
        state.view_mode = ViewMode::ProjectTree;
        state.refresh_project_tree();
        state
            .project_tree_state
            .set_changed_files(vec![changed_file("README.md"), changed_file("src/main.rs")]);

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| crate::ui::render(frame, &mut state))
            .unwrap();

        let output: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(output.contains("Autres fichiers") || output.contains("Other files"));
    }

    fn changed_file(path: &str) -> DiffFile {
        DiffFile {
            path: path.to_string(),
            status: DiffStatus::Modified,
            old_path: None,
            additions: 1,
            deletions: 0,
        }
    }
}
