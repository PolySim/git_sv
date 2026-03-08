//! Liste des fichiers modifiés par un commit (panneau bas-gauche).

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::git::diff::{DiffFile, DiffStatus};
use crate::git::repo::StatusEntry;
use crate::i18n::text_owned;
use crate::state::BottomLeftMode;
use crate::ui::theme::{current_theme, Theme};

/// Contexte de rendu du panneau de fichiers.
pub struct FilesRenderContext<'a> {
    pub commit_files: &'a [DiffFile],
    pub status_entries: &'a [StatusEntry],
    pub selected_commit_hash: Option<&'a str>,
    pub mode: BottomLeftMode,
    pub area: Rect,
    pub is_focused: bool,
    pub file_selected_index: usize,
}

/// Rend le panneau de fichiers dans la zone donnée.
///
/// Affiche soit les fichiers du commit sélectionné, soit le status du working directory
/// selon le mode actif.
pub fn render(frame: &mut Frame, ctx: FilesRenderContext<'_>) {
    let FilesRenderContext {
        commit_files,
        status_entries,
        selected_commit_hash,
        mode,
        area,
        is_focused,
        file_selected_index,
    } = ctx;

    let theme = current_theme();
    let (items, title) = match mode {
        BottomLeftMode::Files => {
            let items = build_commit_file_items(commit_files, theme);
            let hash = selected_commit_hash.unwrap_or("???");
            let title = text_owned(
                format!(" Fichiers - {} ", hash),
                format!(" Files - {} ", hash),
            );
            (items, title)
        }
        BottomLeftMode::Parents => {
            let items = build_status_items(status_entries, theme);
            let title = text_owned(
                format!(" Statut ({} fichiers) ", status_entries.len()),
                format!(" Status ({} files) ", status_entries.len()),
            );
            (items, title)
        }
    };

    let border_style = if is_focused {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };

    let title = if is_focused {
        format!(">{}<", title)
    } else {
        title
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(file_selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Construit les items pour les fichiers d'un commit.
fn build_commit_file_items<'a>(files: &'a [DiffFile], theme: &Theme) -> Vec<ListItem<'a>> {
    files
        .iter()
        .map(|file| {
            let status_char = file.status.display_char();
            let (additions, deletions) = (file.additions, file.deletions);

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_char),
                    Style::default().fg(get_diff_status_color(&file.status, theme)),
                ),
                Span::styled(
                    format!("+{:3} ", additions),
                    Style::default().fg(theme.success),
                ),
                Span::styled(
                    format!("-{:3} ", deletions),
                    Style::default().fg(theme.error),
                ),
                Span::styled(&file.path, Style::default().fg(theme.text_normal)),
            ]);

            ListItem::new(line)
        })
        .collect()
}

/// Construit les items pour le status du working directory.
fn build_status_items<'a>(entries: &'a [StatusEntry], theme: &Theme) -> Vec<ListItem<'a>> {
    entries
        .iter()
        .map(|entry| {
            let status_color = match entry.display_status() {
                "Nouveau (staged)" | "Modifié (staged)" | "Supprimé (staged)" => theme.success,
                "Modifié" | "Supprimé" => theme.error,
                "Non suivi" => theme.text_secondary,
                _ => theme.text_normal,
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:>18} ", entry.display_status()),
                    Style::default().fg(status_color),
                ),
                Span::styled(&entry.path, Style::default().fg(theme.text_normal)),
            ]);
            ListItem::new(line)
        })
        .collect()
}

/// Retourne la couleur pour un statut de diff.
fn get_diff_status_color(status: &DiffStatus, theme: &Theme) -> ratatui::style::Color {
    match status {
        DiffStatus::Added => theme.success,
        DiffStatus::Modified => theme.warning,
        DiffStatus::Deleted => theme.error,
        DiffStatus::Renamed => theme.primary,
    }
}
