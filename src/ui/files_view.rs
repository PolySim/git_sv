use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::BottomLeftMode;
use crate::git::diff::{DiffFile, DiffStatus};
use crate::git::repo::StatusEntry;

/// Rend le panneau de fichiers dans la zone donnée.
///
/// Affiche soit les fichiers du commit sélectionné, soit le status du working directory
/// selon le mode actif.
pub fn render(
    frame: &mut Frame,
    commit_files: &[DiffFile],
    status_entries: &[StatusEntry],
    selected_commit_hash: Option<String>,
    mode: BottomLeftMode,
    area: Rect,
) {
    let (items, title) = match mode {
        BottomLeftMode::CommitFiles => {
            let items = build_commit_file_items(commit_files);
            let hash = selected_commit_hash.unwrap_or_else(|| "???".to_string());
            let title = format!(" Fichiers — {} ", hash);
            (items, title)
        }
        BottomLeftMode::WorkingDir => {
            let items = build_status_items(status_entries);
            let title = format!(" Status ({} fichiers) ", status_entries.len());
            (items, title)
        }
    };

    let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));

    frame.render_widget(list, area);
}

/// Construit les items pour les fichiers d'un commit.
fn build_commit_file_items(files: &[DiffFile]) -> Vec<ListItem> {
    files
        .iter()
        .map(|file| {
            let status_char = file.status.display_char();
            let (additions, deletions) = (file.additions, file.deletions);

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_char),
                    Style::default().fg(get_diff_status_color(&file.status)),
                ),
                Span::styled(
                    format!("+{:3} ", additions),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("-{:3} ", deletions),
                    Style::default().fg(Color::Red),
                ),
                Span::raw(&file.path),
            ]);

            ListItem::new(line)
        })
        .collect()
}

/// Construit les items pour le status du working directory.
fn build_status_items(entries: &[StatusEntry]) -> Vec<ListItem> {
    entries
        .iter()
        .map(|entry| {
            let status_color = match entry.display_status() {
                "Nouveau (staged)" | "Modifié (staged)" | "Supprimé (staged)" => Color::Green,
                "Modifié" | "Supprimé" => Color::Red,
                "Non suivi" => Color::DarkGray,
                _ => Color::White,
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:>18} ", entry.display_status()),
                    Style::default().fg(status_color),
                ),
                Span::raw(&entry.path),
            ]);
            ListItem::new(line)
        })
        .collect()
}

/// Retourne la couleur pour un statut de diff.
fn get_diff_status_color(status: &DiffStatus) -> Color {
    match status {
        DiffStatus::Added => Color::Green,
        DiffStatus::Modified => Color::Yellow,
        DiffStatus::Deleted => Color::Red,
        DiffStatus::Renamed => Color::Cyan,
    }
}
