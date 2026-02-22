use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::git::repo::StatusEntry;
use crate::state::GraphFilter;
use crate::ui::theme::current_theme;

/// Rend la status bar en haut de l'écran.
pub fn render(
    frame: &mut Frame,
    current_branch: &Option<String>,
    _repo_path: &str,
    status_entries: &[StatusEntry],
    flash_message: Option<&str>,
    filter: &GraphFilter,
    area: Rect,
) {
    let theme = current_theme();
    let branch = current_branch.as_deref().unwrap_or("???");

    // Compter les fichiers modifiés/staged/untracked.
    let (modified, staged, untracked) = count_status(status_entries);

    // Construire le statut.
    let status_text = if modified == 0 && staged == 0 && untracked == 0 {
        Span::styled("✓ clean", Style::default().fg(theme.success))
    } else {
        let mut parts = Vec::new();
        if staged > 0 {
            parts.push(format!("{} staged", staged));
        }
        if modified > 0 {
            parts.push(format!("{} modifiés", modified));
        }
        if untracked > 0 {
            parts.push(format!("{} non suivi", untracked));
        }
        Span::styled(
            format!("✗ {}", parts.join(", ")),
            Style::default().fg(theme.error),
        )
    };

    // Construire la ligne.
    let mut spans = vec![
        Span::styled("git_sv  ", Style::default().fg(theme.primary)),
        Span::styled(format!("{}  ", branch), Style::default().fg(theme.commit_hash)),
        status_text,
    ];

    // Ajouter l'indicateur de filtre actif s'il y en a un.
    if filter.is_active() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            "[FILTRÉ]",
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
