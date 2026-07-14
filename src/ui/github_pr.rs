//! Overlay résumant la pull request GitHub de la branche courante.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::git::github::GithubPullRequest;
use crate::ui::common::centered_rect;
use crate::ui::theme::current_theme;

pub struct GithubPrRenderContext<'a> {
    pub pull_request: &'a GithubPullRequest,
    pub area: Rect,
}

/// Rend les informations principales de la PR dans un overlay.
pub fn render(frame: &mut Frame, ctx: GithubPrRenderContext<'_>) {
    let theme = current_theme();
    let popup = centered_rect(76, 62, ctx.area);
    frame.render_widget(Clear, popup);
    let pull_request = ctx.pull_request;
    let draft = if pull_request.is_draft {
        " · brouillon"
    } else {
        ""
    };
    let title = format!(
        " GitHub PR #{} · {}{} ",
        pull_request.number, pull_request.state, draft
    );
    let checks = pull_request.checks;

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            pull_request.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        detail_line("Revue", pull_request.review_label()),
        detail_line("Fusion", pull_request.merge_label()),
        detail_line(
            &format!("Checks ({})", checks.total()),
            &format!(
                "{} réussis · {} échoués · {} en cours · {} ignorés",
                checks.passed, checks.failed, checks.pending, checks.skipped
            ),
        ),
        detail_line(
            "Diff",
            &format!(
                "{} fichiers · +{} / -{}",
                pull_request.changed_files, pull_request.additions, pull_request.deletions
            ),
        ),
        Line::from(""),
        Line::from(Span::styled(
            pull_request.url.clone(),
            Style::default().fg(theme.primary),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Esc ou O pour fermer",
            Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    frame.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.primary)),
            )
            .style(Style::default().bg(theme.background).fg(theme.text_normal))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn detail_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<9}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.to_string()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::github::CheckSummary;

    #[test]
    fn test_detail_lines_include_labels() {
        let pull_request = GithubPullRequest {
            number: 7,
            title: "Improve TUI".to_string(),
            state: "OPEN".to_string(),
            is_draft: false,
            review_decision: Some("APPROVED".to_string()),
            merge_state_status: Some("CLEAN".to_string()),
            url: "https://github.com/acme/repo/pull/7".to_string(),
            additions: 10,
            deletions: 2,
            changed_files: 3,
            checks: CheckSummary {
                passed: 2,
                failed: 0,
                pending: 1,
                skipped: 0,
            },
        };

        assert_eq!(pull_request.review_label(), "approuvée");
        assert!(detail_line("Checks", "2 réussis")
            .to_string()
            .contains("Checks"));
    }
}
