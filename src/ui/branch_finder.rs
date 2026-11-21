//! Rendu du sélecteur temporaire de branches.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::i18n::text;
use crate::state::BranchesViewState;
use crate::ui::common::centered_rect_fixed;
use crate::ui::theme::current_theme;

pub(super) fn popup_area(area: Rect) -> Rect {
    centered_rect_fixed(68, 17, area)
}

pub(super) fn render(frame: &mut Frame, state: &BranchesViewState) {
    let Some(finder) = state.finder() else { return };
    let popup = popup_area(frame.area());
    let theme = current_theme();
    let block = Block::default()
        .title(text(" Trouver une branche ", " Find a branch "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().fg(theme.text_normal).bg(theme.background));
    let inner = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    // Garder une ligne de résultat : sacrifier le compteur, puis le pied,
    // puis la requête si la hauteur disponible diminue.
    let [query, count, results, footer] = Layout::vertical([
        Constraint::Length(u16::from(inner.height >= 2)),
        Constraint::Length(u16::from(inner.height >= 4)),
        Constraint::Min(0),
        Constraint::Length(u16::from(inner.height >= 3)),
    ])
    .areas(inner);

    render_query(frame, query, &finder.query);
    if count.height > 0 {
        let total = finder.matches.len();
        let label = if total == 1 {
            text("résultat", "match")
        } else {
            text("résultats", "matches")
        };
        frame.render_widget(Paragraph::new(format!("{total} {label}")), count);
    }
    render_results(frame, results, state);
    if footer.height > 0 {
        render_footer(frame, footer);
    }
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let full = Line::raw(text(
        "→ : sélectionner  •  ↑↓ : parcourir  •  Esc : annuler",
        "→: select  •  ↑↓: browse  •  Esc: cancel",
    ));
    let compact = Line::raw(text(
        "→ choisir  •  ↑↓  •  Esc annuler",
        "→ select  •  ↑↓  •  Esc cancel",
    ));
    let width = usize::from(area.width);
    let line = if full.width() <= width {
        full
    } else if compact.width() <= width {
        compact
    } else {
        Line::raw(text("→ choisir", "→ select"))
    };
    frame.render_widget(
        Paragraph::new(line).style(Style::default().fg(current_theme().text_secondary)),
        area,
    );
}

fn render_query(frame: &mut Frame, area: Rect, query: &str) {
    if area.width <= 2 || area.height == 0 {
        return;
    }
    let mut line = Line::from(Span::styled(
        "/ ",
        Style::default().fg(current_theme().primary),
    ));
    let input = Line::raw(crate::utils::terminal_text(query).into_owned());
    let available = usize::from(area.width) - line.width();
    let width = input.width();
    let left = width.saturating_sub(available - 1);
    let cursor = (width - left) as u16;
    line.spans
        .extend(super::text_edit::clip_line(input, left, available).spans);
    frame.render_widget(Paragraph::new(line), area);
    frame.set_cursor_position((area.x + 2 + cursor, area.y));
}

fn render_results(frame: &mut Frame, area: Rect, state: &BranchesViewState) {
    let Some(finder) = state.finder() else { return };
    let theme = current_theme();
    let height = usize::from(area.height);
    let offset = finder
        .selected
        .unwrap_or(0)
        .saturating_sub(height.saturating_sub(1));
    let rows = finder
        .matches
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
        .filter_map(|(index, &target)| {
            let branch = state.branch_at(target)?;
            let selected = finder.selected == Some(index);
            let marker = if selected { "› " } else { "  " };
            let kind = if target.is_remote() {
                text("[distante] ", "[remote] ")
            } else {
                text("[locale] ", "[local] ")
            };
            let line = Line::from(vec![
                Span::raw(marker),
                Span::raw(kind),
                Span::raw(crate::utils::terminal_text(&branch.name)),
            ]);
            Some(if selected {
                line.style(
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                line
            })
        })
        .collect::<Vec<_>>();
    let results = if finder.matches.is_empty() {
        Paragraph::new(text("Aucune branche trouvée", "No branches found"))
            .style(Style::default().fg(theme.text_secondary))
    } else {
        Paragraph::new(rows)
    };
    frame.render_widget(results, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    use crate::git::branch::BranchInfo;
    use crate::i18n::{with_language, Language};
    use crate::state::action::BranchFinderAction::*;
    use crate::ui::branches_view::{self, BranchesRenderContext};

    fn render_view(frame: &mut Frame, state: &BranchesViewState) {
        branches_view::render(
            frame,
            BranchesRenderContext {
                state,
                current_branch: Some("topic/00"),
                repo_path: "repo",
                flash_message: None,
                unresolved_conflicts: 0,
            },
        );
    }

    #[test]
    fn finder_renders_live_count_navigation_and_empty_state_at_80x24_in_fr_and_en() {
        for language in [Language::Fr, Language::En] {
            with_language(language, || {
                let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
                let mut state = BranchesViewState::new();
                state.replace_branches(
                    (0..25)
                        .map(|i| BranchInfo::simple(format!("topic/{i:02}"), i == 0, false))
                        .collect(),
                    vec![BranchInfo::simple("origin/topic/26".into(), false, true)],
                );
                let mut draw = |state: &BranchesViewState| {
                    terminal.draw(|frame| render_view(frame, state)).unwrap();
                    let buffer = terminal.backend().buffer();
                    (0..24)
                        .map(|y| (0..80).map(|x| buffer[(x, y)].symbol()).collect::<String>())
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                state.apply_finder(Open);
                state.apply_finder(Insert("TOPIC".into()));
                let screen = draw(&state);
                assert!(screen.contains(text("/:trouver", "/:find")));
                assert!(screen.contains("/ TOPIC"));
                assert!(screen.contains(text("25 résultats", "25 matches")));
                assert!(screen.contains(text("› [locale] topic/00", "› [local] topic/00")));
                assert!(!screen.contains("origin/topic/26"));
                assert!(screen.contains(text("→ : sélectionner", "→: select")));
                assert!(screen.contains(text("Esc : annuler", "Esc: cancel")));

                state.toggle_remote();
                state.apply_finder(Last);
                let screen = draw(&state);
                assert!(screen.contains(text("26 résultats", "26 matches")));
                assert!(screen.contains(text(
                    "› [distante] origin/topic/26",
                    "› [remote] origin/topic/26"
                )));
                state.apply_finder(Accept);
                let screen = draw(&state);
                assert!(!screen.contains(text("Trouver une branche", "Find a branch")));
                assert!(screen.contains("origin/topic/26"));

                state.apply_finder(Open);
                state.apply_finder(Insert("é".repeat(100)));
                let screen = draw(&state);
                assert!(screen.contains(text("0 résultats", "0 matches")));
                assert!(screen.contains(text("Aucune branche trouvée", "No branches found")));
                assert!(screen.contains(text("Esc : annuler", "Esc: cancel")));
            });
        }
    }

    #[test]
    fn finder_keeps_query_selected_result_and_compact_footer_at_40x5() {
        for language in [Language::Fr, Language::En] {
            with_language(language, || {
                let mut terminal = Terminal::new(TestBackend::new(40, 5)).unwrap();
                let mut state = BranchesViewState::new();
                state.replace_branches(
                    ["topic/first", "topic/last"]
                        .into_iter()
                        .map(|name| BranchInfo::simple(name.into(), false, false))
                        .collect(),
                    vec![],
                );
                state.apply_finder(Open);
                state.apply_finder(Insert("topic".into()));
                state.apply_finder(Last);
                terminal.draw(|frame| render_view(frame, &state)).unwrap();

                let buffer = terminal.backend().buffer();
                let row = |y| (0..40).map(|x| buffer[(x, y)].symbol()).collect::<String>();
                assert!(row(1).contains("/ topic"));
                assert!(row(2).contains(text("› [locale] topic/last", "› [local] topic/last")));
                assert!(row(3).contains(text("→ choisir", "→ select")));
                assert!(row(3).contains(text("Esc annuler", "Esc cancel")));
            });
        }
    }
}
