//! Actions en lecture seule du parcours A → B.

use crate::git::diff::{compare_commit_file, compare_commit_files};
use crate::i18n::text;
use crate::state::action::ComparisonAction;
use crate::state::comparison::ComparisonSession;
use crate::state::{AppState, ViewMode};

pub(super) fn handle(state: &mut AppState, action: ComparisonAction) {
    use ComparisonAction::*;

    match (state.view_mode, action) {
        (ViewMode::Graph, Mark) => match state.selected_commit() {
            Some(commit) => state.comparison_base = Some(commit.oid),
            None => no_selection(state),
        },
        (ViewMode::Graph, Clear) => state.comparison_base = None,
        (ViewMode::Graph, Open) => open(state),
        (ViewMode::Comparison, Close) => {
            state.comparison = None;
            state.image_preview.clear();
            state.return_from_comparison();
        }
        (ViewMode::Comparison, _) => {
            if let Some(session) = &mut state.comparison {
                navigate(&state.repo.repo, session, action);
            }
        }
        _ => {}
    }
}

fn no_selection(state: &mut AppState) {
    state.set_flash_message(text("Aucun commit sélectionné", "No commit selected"));
}

fn open(state: &mut AppState) {
    let Some(a) = state.comparison_base else {
        state.set_flash_message(text("Marquez d’abord A avec a", "Mark A with a first"));
        return;
    };
    let Some(b) = state.selected_commit().map(|commit| commit.oid) else {
        no_selection(state);
        return;
    };
    let repo = &state.repo.repo;
    let mut session = ComparisonSession::new(a, b, compare_commit_files(repo, a, b));
    load_file(repo, &mut session);
    state.comparison = Some(session);
    state.enter_view(ViewMode::Comparison);
}

fn load_file(repo: &git2::Repository, session: &mut ComparisonSession) {
    session.scroll = 0;
    session.horizontal = 0;
    session.max_scroll = 0;
    session.max_horizontal = 0;
    session.diff = session
        .files
        .as_ref()
        .ok()
        .and_then(|files| files.get(session.selected))
        .map(|file| compare_commit_file(repo, file));
    let Some(Ok(detail)) = &session.diff else {
        session.content_width = 0;
        return;
    };
    let metadata_width = detail
        .metadata
        .iter()
        .map(|line| ratatui::text::Span::raw(line.as_str()).width())
        .max()
        .unwrap_or_default();
    let diff_width = if detail.diff.image_preview.is_none() {
        crate::ui::diff_view::unified_content_width(&detail.diff)
    } else {
        0
    };
    session.content_width = metadata_width.max(diff_width);
}

fn navigate(repo: &git2::Repository, session: &mut ComparisonSession, action: ComparisonAction) {
    use ComparisonAction::*;

    match action {
        FileUp | FileDown => {
            let Ok(files) = &session.files else { return };
            let next = if action == FileUp {
                session.selected.saturating_sub(1)
            } else {
                (session.selected + 1).min(files.len().saturating_sub(1))
            };
            // Pas de nouveau calcul à chaque rendu, ni si la sélection ne bouge pas.
            if next != session.selected {
                session.selected = next;
                load_file(repo, session);
            }
        }
        LineUp => session.scroll = session.scroll.saturating_sub(1),
        LineDown => session.scroll = (session.scroll + 1).min(session.max_scroll),
        PageUp => session.scroll = session.scroll.saturating_sub(session.page_height),
        PageDown => session.scroll = (session.scroll + session.page_height).min(session.max_scroll),
        Left => session.horizontal = session.horizontal.saturating_sub(4),
        Right => session.horizontal = (session.horizontal + 4).min(session.max_horizontal),
        Mark | Clear | Open | Close => {}
    }
}
