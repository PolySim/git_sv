//! Overlay de diagnostic local du dépôt.

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::git::insights::{CommitSignatureStatus, RepositoryInsights, SubmoduleState};
use crate::ui::common::centered_rect;
use crate::ui::theme::current_theme;

pub struct RepositoryInsightsRenderContext<'a> {
    pub insights: &'a RepositoryInsights,
    pub scroll: u16,
    pub area: Rect,
}

/// Rend le diagnostic dans un overlay défilable.
pub fn render(frame: &mut Frame, ctx: RepositoryInsightsRenderContext<'_>) {
    let theme = current_theme();
    let popup = centered_rect(82, 84, ctx.area);
    frame.render_widget(Clear, popup);

    let short_commit = ctx.insights.commit.chars().take(7).collect::<String>();
    let content = build_content(ctx.insights);
    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!(" Diagnostic dépôt · {short_commit} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        )
        .style(Style::default().bg(theme.background).fg(theme.text_normal))
        .scroll((ctx.scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
}

fn build_content(insights: &RepositoryInsights) -> Vec<Line<'static>> {
    let theme = current_theme();
    let mut lines = vec![Line::from("")];

    lines.push(section("Signature du commit"));
    let (symbol, color) = match insights.signature {
        CommitSignatureStatus::Verified { .. } => ("✓", theme.success),
        CommitSignatureStatus::Unsigned => ("○", theme.text_secondary),
        CommitSignatureStatus::UnknownKey { .. } | CommitSignatureStatus::Present { .. } => {
            ("!", theme.warning)
        }
        CommitSignatureStatus::Invalid { .. } => ("✗", theme.error),
    };
    lines.push(Line::from(vec![
        Span::styled(format!("  {symbol} "), Style::default().fg(color)),
        Span::raw(insights.signature.summary()),
    ]));
    lines.push(Line::from(""));

    lines.push(section(format!(
        "Hooks · {} actif(s) / {} configuré(s)",
        insights.enabled_hook_count(),
        insights.hooks.len()
    )));
    if insights.hooks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Aucun hook personnalisé",
            Style::default().fg(theme.text_secondary),
        )));
    } else {
        for hook in &insights.hooks {
            let (symbol, suffix, color) = if hook.enabled {
                ("✓", "", theme.success)
            } else {
                ("○", " · non exécutable", theme.text_secondary)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {symbol} "), Style::default().fg(color)),
                Span::raw(hook.name.clone()),
                Span::styled(suffix, Style::default().fg(theme.text_secondary)),
            ]));
        }
    }
    lines.push(Line::from(""));

    lines.push(section(format!(
        "Sous-modules · {} · {} à vérifier",
        insights.submodules.len(),
        insights.dirty_submodule_count()
    )));
    if insights.submodules.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Aucun sous-module",
            Style::default().fg(theme.text_secondary),
        )));
    } else {
        for submodule in &insights.submodules {
            let (symbol, color) = match submodule.state {
                SubmoduleState::Clean => ("✓", theme.success),
                SubmoduleState::Modified => ("!", theme.warning),
                SubmoduleState::Uninitialized => ("○", theme.text_secondary),
            };
            let revision = submodule.revision.as_deref().unwrap_or("-------");
            lines.push(Line::from(vec![
                Span::styled(format!("  {symbol} "), Style::default().fg(color)),
                Span::styled(
                    submodule.path.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" · {revision} · {}", submodule.state.label())),
            ]));
            if let Some(url) = &submodule.url {
                lines.push(Line::from(Span::styled(
                    format!("      {url}"),
                    Style::default().fg(theme.text_secondary),
                )));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k défiler · Esc ou i fermer",
        Style::default()
            .fg(theme.text_secondary)
            .add_modifier(Modifier::ITALIC),
    )));
    lines
}

fn section(title: impl Into<String>) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {} ", title.into()),
        Style::default().add_modifier(Modifier::BOLD),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::insights::{HookInfo, SubmoduleInfo};

    #[test]
    fn test_content_contains_all_diagnostic_sections() {
        let insights = RepositoryInsights {
            commit: "0123456789012345678901234567890123456789".to_string(),
            signature: CommitSignatureStatus::Unsigned,
            hooks: vec![HookInfo {
                name: "pre-commit".to_string(),
                enabled: true,
            }],
            submodules: vec![SubmoduleInfo {
                name: "lib".to_string(),
                path: "vendor/lib".to_string(),
                url: Some("https://example.com/lib.git".to_string()),
                revision: Some("abcdef0".to_string()),
                state: SubmoduleState::Clean,
            }],
        };

        let content = build_content(&insights)
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(content.contains("Signature"));
        assert!(content.contains("pre-commit"));
        assert!(content.contains("vendor/lib"));
    }
}
