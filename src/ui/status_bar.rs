//! Barre de statut en haut de l'écran (branche, chemin repo, message flash).

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::git::repo::StatusEntry;
use crate::i18n::{text, text_owned};
use crate::state::GraphFilter;
use crate::ui::theme::current_theme;

/// Contexte de rendu de la barre de statut principale.
pub struct StatusBarRenderContext<'a> {
    pub current_branch: Option<&'a str>,
    pub status_entries: &'a [StatusEntry],
    pub flash_message: Option<&'a str>,
    pub filter: &'a GraphFilter,
    pub is_merging: bool,
    pub area: Rect,
}

/// Rend la status bar en haut de l'écran.
pub fn render(frame: &mut Frame, ctx: StatusBarRenderContext<'_>) {
    let StatusBarRenderContext {
        current_branch,
        status_entries,
        flash_message,
        filter,
        is_merging,
        area,
    } = ctx;

    let theme = current_theme();
    let branch = current_branch.unwrap_or(text("???", "???"));

    // Compter les fichiers modifiés/staged/untracked.
    let (modified, staged, untracked) = count_status(status_entries);

    // Construire le statut.
    let status_text = if modified == 0 && staged == 0 && untracked == 0 {
        Span::styled(
            text("✓ propre", "✓ clean"),
            Style::default().fg(theme.success),
        )
    } else {
        let mut parts = Vec::new();
        if staged > 0 {
            parts.push(text_owned(
                format!("{} indexes", staged),
                format!("{} staged", staged),
            ));
        }
        if modified > 0 {
            parts.push(text_owned(
                format!("{} modifies", modified),
                format!("{} modified", modified),
            ));
        }
        if untracked > 0 {
            parts.push(text_owned(
                format!("{} non suivis", untracked),
                format!("{} untracked", untracked),
            ));
        }
        Span::styled(
            format!("✗ {}", parts.join(", ")),
            Style::default().fg(theme.error),
        )
    };

    // Construire la ligne.
    let mut spans = vec![
        Span::styled("git_sv  ", Style::default().fg(theme.primary)),
        Span::styled(
            format!("{}  ", branch),
            Style::default().fg(theme.commit_hash),
        ),
        status_text,
    ];

    // Ajouter l'indicateur MERGING si un merge est en cours.
    if is_merging {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            text("⚠ FUSION", "⚠ MERGING"),
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Ajouter l'indicateur de filtre actif s'il y en a un.
    if filter.is_active() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            text("[FILTRE]", "[FILTERED]"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Ajouter le message flash s'il existe.
    if let Some(msg) = flash_message {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            msg.to_string(),
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

/// Compte les fichiers par catégorie.
fn count_status(entries: &[StatusEntry]) -> (usize, usize, usize) {
    let mut modified = 0;
    let mut staged = 0;
    let mut untracked = 0;

    for entry in entries {
        let s = entry.status;
        if s.contains(git2::Status::WT_MODIFIED) || s.contains(git2::Status::WT_DELETED) {
            modified += 1;
        }
        if s.contains(git2::Status::INDEX_NEW)
            || s.contains(git2::Status::INDEX_MODIFIED)
            || s.contains(git2::Status::INDEX_DELETED)
        {
            staged += 1;
        }
        if s.contains(git2::Status::WT_NEW) {
            untracked += 1;
        }
    }

    (modified, staged, untracked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{with_language, Language};

    #[test]
    fn test_clean_label_is_localized_in_english() {
        with_language(Language::En, || {
            let theme = current_theme();
            let label = if true {
                Span::styled(
                    text("✓ propre", "✓ clean"),
                    Style::default().fg(theme.success),
                )
            } else {
                unreachable!()
            };

            assert_eq!(label.content, "✓ clean");
        });
    }
}
