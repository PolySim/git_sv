//! Surface de lecture dédiée à la comparaison de deux arbres de commits.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::git::diff::{DiffStatus, DiffViewMode};
use crate::i18n::{text, text_owned};
use crate::state::comparison::ComparisonSession;
use crate::state::AppState;
use crate::ui::common::help_bar::{self, KeyHint};
use crate::ui::{diff_view, layout, theme::current_theme};
use crate::utils::{terminal_path, terminal_text};

pub(super) fn render(frame: &mut Frame, state: &mut AppState) {
    let Some(session) = &mut state.comparison else {
        return;
    };
    let layout = layout::build_layout_with_diff_mode(frame.area(), false, true);
    let main = layout.diff_fullscreen.unwrap_or_default();
    let main = Rect::new(
        main.x,
        layout.nav_bar.y,
        main.width,
        main.height + layout.nav_bar.height,
    );
    let [files, diff] = if main.width >= 68 {
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(main)
    } else {
        Layout::vertical([
            Constraint::Length((main.height / 3).clamp(3, 7)),
            Constraint::Min(0),
        ])
        .areas(main)
    };

    let title = format!(
        "A → B  {:.7} → {:.7}",
        session.a.to_string(),
        session.b.to_string()
    );
    frame.render_widget(
        Paragraph::new(title).style(
            Style::default()
                .fg(current_theme().primary)
                .add_modifier(Modifier::BOLD),
        ),
        layout.status_bar,
    );
    render_files(frame, files, session);
    render_diff(frame, diff, session, &mut state.image_preview);
    help_bar::render(
        frame,
        layout.help_bar,
        &[
            KeyHint::new("Esc", text("graphe", "graph")),
            KeyHint::new("↑↓", text("fichiers", "files")),
            KeyHint::new("j/k · Pg↑↓", "diff"),
            KeyHint::new("←→", text("horizontal", "horizontal")),
        ],
        None,
    );
}

fn render_files(frame: &mut Frame, area: Rect, session: &mut ComparisonSession) {
    let theme = current_theme();
    let title = text("Fichiers", "Files");
    let files = match &session.files {
        Ok(files) if !files.is_empty() => files,
        Ok(_) => {
            return message(
                frame,
                area,
                title,
                text("Aucun fichier modifié", "No changed files"),
            )
        }
        Err(error) => return message(frame, area, title, &error_text(error)),
    };
    let height = usize::from(area.height.saturating_sub(2));
    if height > 0 {
        session.file_offset = session
            .file_offset
            .max((session.selected + 1).saturating_sub(height))
            .min(session.selected)
            .min(files.len().saturating_sub(height));
    }
    let rows = files
        .iter()
        .skip(session.file_offset)
        .take(height)
        .map(|file| {
            let status = match file.status {
                DiffStatus::Added => "A",
                DiffStatus::Modified => "M",
                DiffStatus::Deleted => "D",
                DiffStatus::Renamed => "R",
            };
            let path = file.path();
            let path = terminal_path(&path);
            match file.old_path() {
                Some(old) => format!("{status} {} → {path}", terminal_path(&old)),
                None => format!("{status} {path}"),
            }
        });
    let block = panel(format!("{title} {}/{}", session.selected + 1, files.len()));
    let list = List::new(rows)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    let mut selection =
        ListState::default().with_selected(Some(session.selected - session.file_offset));
    frame.render_stateful_widget(list, area, &mut selection);
}

fn render_diff(
    frame: &mut Frame,
    area: Rect,
    session: &mut ComparisonSession,
    image_state: &mut crate::ui::image_preview::ImagePreviewState,
) {
    session.max_horizontal = session
        .content_width
        .saturating_sub(usize::from(area.width.saturating_sub(2)));
    session.horizontal = session.horizontal.min(session.max_horizontal);
    let area = match &session.diff {
        Some(Ok(detail)) if !detail.metadata.is_empty() => {
            let [metadata, body] = Layout::vertical([
                Constraint::Length(detail.metadata.len() as u16),
                Constraint::Min(0),
            ])
            .areas(area);
            frame.render_widget(
                Paragraph::new(detail.metadata.join("\n"))
                    .style(Style::default().fg(current_theme().info))
                    .scroll((0, session.horizontal.min(usize::from(u16::MAX)) as u16)),
                metadata,
            );
            body
        }
        _ => area,
    };
    let diff = match session
        .diff
        .as_ref()
        .map(|result| result.as_ref().map(|detail| &detail.diff))
    {
        Some(Ok(diff)) if !diff.lines.is_empty() || diff.image_preview.is_some() => diff,
        result => {
            image_state.clear();
            let notice = match result {
                Some(Ok(_)) => text("Aucun changement de contenu", "No content changes").into(),
                Some(Err(error)) => error_text(error),
                None => String::new(),
            };
            return message(frame, area, "Diff", &notice);
        }
    };
    session.page_height = usize::from(area.height.saturating_sub(2)).max(1);
    session.max_scroll = diff.lines.len().saturating_sub(session.page_height);
    session.scroll = session.scroll.min(session.max_scroll);
    diff_view::render(
        frame,
        diff_view::DiffRenderContext {
            diff: Some(diff),
            scroll_offset: session.scroll,
            horizontal_offset: session.horizontal,
            area,
            is_focused: false,
            view_mode: DiffViewMode::Unified,
            is_fullscreen: false,
            image_state,
            selected_line: None,
        },
    );
}

fn panel(title: impl Into<ratatui::text::Line<'static>>) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(current_theme().border_inactive))
}

fn message(frame: &mut Frame, area: Rect, title: &'static str, content: &str) {
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .block(panel(title)),
        area,
    );
}

fn error_text(error: &crate::error::GitSvError) -> String {
    let detail = error.to_string();
    let detail = terminal_text(&detail);
    text_owned(
        format!("Comparaison indisponible : {detail}"),
        format!("Comparison unavailable: {detail}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ViewMode;
    use crate::test_utils::ui_driver::UiTestHarness;
    use crate::ui::tests::render_to_string;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn draw(harness: &mut UiTestHarness, width: u16) -> String {
        render_to_string(width, 22, |frame| {
            crate::ui::render(frame, &mut harness.state)
        })
    }

    #[test]
    fn comparison_reads_fixed_commits_and_restores_graph_after_refresh() {
        let mut harness = UiTestHarness::new();
        let a = harness.commit_file("alpha.txt", "before\n", "A");
        harness.refresh_graph();
        harness.send_char('a');
        assert_eq!(harness.state.comparison_base, Some(a));
        assert!(draw(&mut harness, 80).contains(&format!("A: {:.7}", a.to_string())));

        harness.commit_file("alpha.txt", "after\n", "intermediate");
        let long = (1..=120)
            .map(|n| format!("line-{n:03} {}\n", "x".repeat(90)))
            .collect::<String>();
        let b = harness.commit_file("beta.txt", &long, "B");
        harness.refresh_graph();
        harness.state.graph_view.select_commit(0);
        harness.state.refresh_commit_files();
        crate::handler::navigation::load_commit_file_diff(&mut harness.state);
        harness.state.graph_view.diff_scroll_offset = 12;
        harness.state.graph_view.diff_horizontal_offset = 8;
        harness.state.focus = crate::state::FocusPanel::BottomLeft;
        harness.write_file("alpha.txt", "staged\n");
        harness.stage_file("alpha.txt");
        harness.write_file("alpha.txt", "dirty worktree\n");
        let index_path = harness.state.repo.repo.path().join("index");
        let index = std::fs::read(&index_path).unwrap();

        harness.send_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
        assert_eq!(harness.state.view_mode, ViewMode::Comparison);
        let output = draw(&mut harness, 48);
        for value in ["A → B", "alpha.txt", "beta.txt", "before", "after", "Esc"] {
            assert!(output.contains(value), "{value} absent de {output}");
        }
        assert!(!output.contains("dirty worktree"));
        assert!(!output.contains("staged"));
        assert_eq!(harness.state.repo.repo.head().unwrap().target(), Some(b));
        assert_eq!(std::fs::read(index_path).unwrap(), index);
        assert_eq!(
            std::fs::read_to_string(harness.state.repo.repo.workdir().unwrap().join("alpha.txt"))
                .unwrap(),
            "dirty worktree\n"
        );

        harness.send_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert!(draw(&mut harness, 80).contains("line-001"));
        harness.send_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        assert!(!draw(&mut harness, 80).contains("line-001"));
        assert!(harness.state.comparison.as_ref().unwrap().scroll > 0);
        harness.send_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert!(draw(&mut harness, 80).contains("before"));
        assert_eq!(harness.state.comparison.as_ref().unwrap().scroll, 0);

        let old_len = harness.state.graph_view.len();
        harness.commit_file("new.txt", "new HEAD\n", "C");
        harness.state.mark_dirty();
        harness.state.refresh_pending().unwrap();
        let session = harness.state.comparison.as_ref().unwrap();
        assert_eq!((session.a, session.b), (a, b));
        assert_eq!(session.files.as_ref().unwrap().len(), 2);
        assert_eq!(harness.state.graph_view.len(), old_len);
        harness.send_esc();
        assert_eq!(harness.state.view_mode, ViewMode::Graph);
        assert_eq!(harness.state.focus, crate::state::FocusPanel::BottomLeft);
        assert!(harness.state.comparison.is_none());
        harness.state.refresh_pending().unwrap();
        assert_eq!(harness.state.graph_view.len(), old_len + 1);
        assert_eq!(harness.state.selected_commit().unwrap().oid, b);
        let graph = &harness.state.graph_view;
        assert_eq!(graph.selected_file_diff.as_ref().unwrap().path, "beta.txt");
        assert_eq!(
            (graph.diff_scroll_offset, graph.diff_horizontal_offset),
            (12, 8)
        );
        assert_eq!(harness.state.comparison_base, Some(a));
        harness.send_char('a');
        assert_eq!(harness.state.comparison_base, Some(b));
        harness.send_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(harness.state.comparison_base, None);
    }

    #[test]
    fn comparison_return_clamps_restored_offsets_to_the_graph_viewport() {
        let mut harness = UiTestHarness::new();
        let a = harness.commit_file("beta.txt", "before\n", "A");
        let long = format!("{}\n", "x".repeat(50)).repeat(24);
        let b = harness.commit_file("beta.txt", &long, "B");
        harness.refresh_graph();
        harness.state.comparison_base = Some(a);
        harness.state.focus = crate::state::FocusPanel::BottomRight;
        harness.state.graph_view.diff_scroll_offset = 200;
        harness.state.graph_view.diff_horizontal_offset = 200;
        harness.send_char('D');
        assert_eq!(harness.state.view_mode, ViewMode::Comparison);
        draw(&mut harness, 80);
        harness.state.schedule_refresh();
        harness.send_esc();
        harness.state.refresh_pending().unwrap();

        let graph = &harness.state.graph_view;
        assert_eq!(harness.state.selected_commit().unwrap().oid, b);
        let diff = graph.selected_file_diff.as_ref().unwrap();
        let panel = layout::build_layout_with_diff_mode(harness.state.screen_area, false, false)
            .bottom_right;
        assert_eq!(
            graph.diff_scroll_offset,
            diff.lines.len() - usize::from(panel.height - 2)
        );
        assert_eq!(
            graph.diff_horizontal_offset,
            61 - usize::from(panel.width - 2)
        );
        assert!(draw(&mut harness, 80).contains("xxxxx"));
    }

    #[test]
    fn comparison_return_does_not_restore_offsets_for_a_different_path() {
        let mut harness = UiTestHarness::new();
        let a = harness.commit_file("a.txt", "before\n", "A");
        let long = format!("{}\n", "x".repeat(90)).repeat(24);
        for path in ["a.txt", "b.txt"] {
            harness.write_file(path, &long);
            harness.stage_file(path);
        }
        let b = harness.commit_all("B");
        harness.refresh_graph();
        harness.state.comparison_base = Some(a);
        harness.state.graph_view.diff_scroll_offset = 4;
        harness.state.graph_view.diff_horizontal_offset = 6;
        harness.send_char('D');
        // Simule un index de fichier qui désigne un autre chemin au rechargement.
        harness.state.graph_view.file_selected_index = 1;
        harness.state.schedule_refresh();
        harness.send_esc();
        harness.state.refresh_pending().unwrap();

        let graph = &harness.state.graph_view;
        assert_eq!(harness.state.selected_commit().unwrap().oid, b);
        assert_eq!(graph.selected_file_diff.as_ref().unwrap().path, "b.txt");
        assert_eq!(
            (graph.diff_scroll_offset, graph.diff_horizontal_offset),
            (0, 0)
        );
    }

    #[test]
    fn comparison_displays_a_mode_only_change() {
        let mut harness = UiTestHarness::new();
        let a = harness.commit_file("script.sh", "exit 0\n", "A");
        {
            use git2::FileMode;

            let repo = &harness.state.repo.repo;
            let parent = repo.find_commit(a).unwrap();
            let old_tree = parent.tree().unwrap();
            let blob = old_tree.get_name("script.sh").unwrap().id();
            let mut tree = repo.treebuilder(Some(&old_tree)).unwrap();
            tree.insert("script.sh", blob, FileMode::BlobExecutable.into())
                .unwrap();
            let tree = repo.find_tree(tree.write().unwrap()).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "chmod", &tree, &[&parent])
                .unwrap();
        }
        harness.refresh_graph();
        harness.state.comparison_base = Some(a);
        harness.state.graph_view.select_commit(0);
        harness.send_char('D');

        let output = draw(&mut harness, 80);
        assert!(output.contains("Mode 100644 → 100755"), "{output}");
        let detail = harness
            .state
            .comparison
            .as_ref()
            .unwrap()
            .diff
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap();
        assert!(detail.diff.lines.is_empty());
    }

    #[test]
    fn comparison_displays_empty_list_and_missing_commit_in_place() {
        let mut harness = UiTestHarness::new();
        harness.refresh_graph();
        harness.send_char('D');
        assert_eq!(harness.state.view_mode, ViewMode::Graph);
        assert!(harness.state.current_flash_message().is_some());
        harness.send_char('a');
        harness.send_char('D');
        let output = draw(&mut harness, 48);
        assert!(output.contains(text("Aucun fichier modifié", "No changed files")));
        harness.send_esc();
        harness.state.comparison_base = Some(git2::Oid::ZERO_SHA1);
        harness.send_char('D');
        assert!(draw(&mut harness, 48)
            .contains(text("Comparaison indisponible", "Comparison unavailable")));
        assert!(harness.state.comparison.as_ref().unwrap().files.is_err());
        harness.send_esc();
        assert_eq!(harness.state.view_mode, ViewMode::Graph);
        assert_eq!(harness.state.comparison_base, Some(git2::Oid::ZERO_SHA1));
    }
}
