use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{StagingFocus, StagingState};
use crate::git::repo::StatusEntry;

/// Rend la vue complète de staging.
pub fn render(
    frame: &mut Frame,
    staging_state: &StagingState,
    current_branch: &Option<String>,
    repo_path: &str,
    flash_message: Option<&str>,
) {
    let layout = super::staging_layout::build_staging_layout(frame.area());

    // Status bar.
    render_staging_status_bar(
        frame,
        current_branch,
        repo_path,
        flash_message,
        layout.status_bar,
    );

    // Panneau unstaged.
    render_file_list(
        frame,
        "Unstaged",
        &staging_state.unstaged_files,
        staging_state.unstaged_selected,
        staging_state.focus == StagingFocus::Unstaged,
        layout.unstaged_panel,
    );

    // Panneau staged.
    render_file_list(
        frame,
        "Staged",
        &staging_state.staged_files,
        staging_state.staged_selected,
        staging_state.focus == StagingFocus::Staged,
        layout.staged_panel,
    );

    // Panneau diff.
    super::diff_view::render(
        frame,
        staging_state.current_diff.as_ref(),
        staging_state.diff_scroll,
        layout.diff_panel,
        staging_state.focus == StagingFocus::Diff,
    );

    // Zone de message commit.
    render_commit_input(
        frame,
        &staging_state.commit_message,
        staging_state.cursor_position,
        staging_state.focus == StagingFocus::CommitMessage,
        !staging_state.staged_files.is_empty(),
        layout.commit_message,
    );

    // Help bar.
    render_staging_help(frame, &staging_state.focus, layout.help_bar);
}

/// Rend la status bar de la vue staging.
fn render_staging_status_bar(
    frame: &mut Frame,
    current_branch: &Option<String>,
    repo_path: &str,
    flash_message: Option<&str>,
    area: Rect,
) {
    let branch_name = current_branch.as_deref().unwrap_or("???");

    let content = if let Some(msg) = flash_message {
        format!(
            " git_sv · staging · {} · {} · {} ",
            repo_path, branch_name, msg
        )
    } else {
        format!(" git_sv · staging · {} · {} ", repo_path, branch_name)
    };

    let line = Line::from(vec![Span::styled(
        content,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Cyan)),
        area,
    );
}

/// Rend une liste de fichiers (unstaged ou staged).
fn render_file_list(
    frame: &mut Frame,
    title: &str,
    files: &[StatusEntry],
    selected: usize,
    is_focused: bool,
    area: Rect,
) {
    let items: Vec<ListItem> = files
        .iter()
        .map(|entry| {
            let status_icon = match entry.display_status() {
                s if s.contains("staged") => "●",
                "Modifié" => "M",
                "Supprimé" => "D",
                "Non suivi" => "?",
                _ => " ",
            };

            let status_color = match entry.display_status() {
                s if s.contains("staged") => Color::Green,
                "Modifié" => Color::Yellow,
                "Supprimé" => Color::Red,
                "Non suivi" => Color::DarkGray,
                _ => Color::White,
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::raw(&entry.path),
            ]);

            ListItem::new(line)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let count = files.len();
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {} ({}) ", title, count))
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Rend le champ de saisie du message de commit.
fn render_commit_input(
    frame: &mut Frame,
    message: &str,
    cursor_pos: usize,
    is_focused: bool,
    has_staged_files: bool,
    area: Rect,
) {
    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let title = if has_staged_files {
        " Message de commit (Enter pour valider) "
    } else {
        " Message de commit (aucun fichier staged) "
    };

    let display_text = if message.is_empty() && !is_focused {
        "Appuyez sur 'c' pour écrire un message de commit..."
    } else {
        message
    };

    let paragraph = Paragraph::new(display_text).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style),
    );

    frame.render_widget(paragraph, area);

    // Si focalisé, positionner le curseur.
    if is_focused {
        // Calculer la position du curseur en tenant compte des retours à la ligne
        let mut cursor_line = 0u16;
        let mut cursor_col = 0u16;
        let mut current_pos = 0usize;

        for char in message.chars() {
            if current_pos >= cursor_pos {
                break;
            }
            if char == '\n' {
                cursor_line += 1;
                cursor_col = 0;
            } else {
                cursor_col += 1;
            }
            current_pos += 1;
        }

        frame.set_cursor_position((area.x + cursor_col + 1, area.y + cursor_line + 1));
    }
}

/// Rend la barre d'aide de la vue staging.
fn render_staging_help(frame: &mut Frame, focus: &StagingFocus, area: Rect) {
    let help_text = match focus {
        StagingFocus::Unstaged => {
            "j/k:nav  s/Enter:stage  a:stage all  Tab:→Staged  c:commit  P:push  1:graph  q:quit"
        }
        StagingFocus::Staged => {
            "j/k:nav  u/Enter:unstage  U:unstage all  Tab:→Diff  c:commit  P:push  1:graph  q:quit"
        }
        StagingFocus::Diff => {
            "j/k:scroll  Tab:→Unstaged  Esc:Unstaged  c:commit  P:push  1:graph  q:quit"
        }
        StagingFocus::CommitMessage => "Enter:confirmer  Esc:annuler  ←→:curseur",
    };

    let line = Line::from(vec![Span::styled(
        format!(" {} ", help_text),
        Style::default().fg(Color::DarkGray),
    )]);

    frame.render_widget(Paragraph::new(line), area);
}
