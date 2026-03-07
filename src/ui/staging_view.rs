//! Rendu de la vue staging (fichiers stagés/non stagés, diff, commit).

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{StagingFocus, StagingState};
use crate::git::repo::StatusEntry;
use crate::ui::common::StatusBarConfig;
use crate::ui::theme::{current_theme, Theme};

pub struct StagingRenderContext<'a> {
    pub staging_state: &'a StagingState,
    pub current_branch: Option<&'a str>,
    pub repo_path: &'a str,
    pub flash_message: Option<&'a str>,
    pub is_merging: bool,
}

struct FileListRenderContext<'a> {
    title: &'a str,
    files: &'a [StatusEntry],
    selected: usize,
    is_focused: bool,
    area: Rect,
    theme: &'a Theme,
}

struct CommitInputRenderContext<'a> {
    message: &'a str,
    cursor_pos: usize,
    is_focused: bool,
    has_staged_files: bool,
    is_amending: bool,
    area: Rect,
    theme: &'a Theme,
}

struct StagingHelpRenderContext<'a> {
    focus: &'a StagingFocus,
    is_merging: bool,
    area: Rect,
    theme: &'a Theme,
}

/// Rend la vue complète de staging.
pub fn render(frame: &mut Frame, ctx: StagingRenderContext<'_>) {
    let StagingRenderContext {
        staging_state,
        current_branch,
        repo_path,
        flash_message,
        is_merging,
    } = ctx;

    let theme = current_theme();
    let layout = super::staging_layout::build_staging_layout(frame.area());

    // Status bar.
    crate::ui::common::render_status_bar(
        frame,
        StatusBarConfig {
            view_title: "staging",
            branch: current_branch,
            repo_path,
            flash_message,
            bg_color: None,
        },
        layout.status_bar,
    );

    // Panneau unstaged.
    render_file_list(
        frame,
        FileListRenderContext {
            title: "Unstaged",
            files: staging_state.unstaged_files(),
            selected: staging_state.unstaged_selected(),
            is_focused: staging_state.focus == StagingFocus::Unstaged,
            area: layout.unstaged_panel,
            theme,
        },
    );

    // Panneau staged.
    render_file_list(
        frame,
        FileListRenderContext {
            title: "Staged",
            files: staging_state.staged_files(),
            selected: staging_state.staged_selected(),
            is_focused: staging_state.focus == StagingFocus::Staged,
            area: layout.staged_panel,
            theme,
        },
    );

    // Panneau diff.
    super::diff_view::render(
        frame,
        super::diff_view::DiffRenderContext {
            diff: staging_state.current_diff.as_ref(),
            scroll_offset: staging_state.diff_scroll,
            horizontal_offset: staging_state.diff_horizontal_offset,
            area: layout.diff_panel,
            is_focused: staging_state.focus == StagingFocus::Diff,
            view_mode: staging_state.diff_view_mode,
            is_fullscreen: false,
        },
    );

    // Zone de message commit.
    render_commit_input(
        frame,
        CommitInputRenderContext {
            message: &staging_state.commit_message,
            cursor_pos: staging_state.cursor_position,
            is_focused: staging_state.focus == StagingFocus::CommitMessage,
            has_staged_files: !staging_state.staged_files().is_empty(),
            is_amending: staging_state.is_amending,
            area: layout.commit_message,
            theme,
        },
    );

    // Help bar.
    render_staging_help(
        frame,
        StagingHelpRenderContext {
            focus: &staging_state.focus,
            is_merging,
            area: layout.help_bar,
            theme,
        },
    );
}

/// Rend une liste de fichiers (unstaged ou staged).
fn render_file_list(frame: &mut Frame, ctx: FileListRenderContext<'_>) {
    let FileListRenderContext {
        title,
        files,
        selected,
        is_focused,
        area,
        theme,
    } = ctx;

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
                s if s.contains("staged") => theme.success,
                "Modifié" => theme.warning,
                "Supprimé" => theme.error,
                "Non suivi" => theme.text_secondary,
                _ => theme.text_normal,
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(&entry.path, Style::default().fg(theme.text_normal)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(theme.primary)
    } else {
        Style::default().fg(theme.border_inactive)
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
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Rend le champ de saisie du message de commit.
fn render_commit_input(frame: &mut Frame, ctx: CommitInputRenderContext<'_>) {
    let CommitInputRenderContext {
        message,
        cursor_pos,
        is_focused,
        has_staged_files,
        is_amending,
        area,
        theme,
    } = ctx;

    let border_style = if is_focused {
        Style::default().fg(theme.warning)
    } else {
        Style::default().fg(theme.border_inactive)
    };

    let title = if is_amending {
        " Amend commit (Enter pour valider) "
    } else if has_staged_files {
        " Message de commit (Enter pour valider) "
    } else {
        " Message de commit (aucun fichier staged) "
    };

    let display_text = if message.is_empty() && !is_focused {
        if is_amending {
            "Appuyez sur 'A' pour amender le dernier commit..."
        } else {
            "Appuyez sur 'c' pour écrire un message de commit..."
        }
    } else {
        message
    };

    let text_style = if message.is_empty() && !is_focused {
        Style::default().fg(theme.text_secondary)
    } else {
        Style::default().fg(theme.text_normal)
    };

    let paragraph = Paragraph::new(display_text).style(text_style).block(
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
        for (current_pos, char) in message.chars().enumerate() {
            if current_pos >= cursor_pos {
                break;
            }
            if char == '\n' {
                cursor_line += 1;
                cursor_col = 0;
            } else {
                cursor_col += 1;
            }
        }

        frame.set_cursor_position((area.x + cursor_col + 1, area.y + cursor_line + 1));
    }
}

/// Rend la barre d'aide de la vue staging.
fn render_staging_help(frame: &mut Frame, ctx: StagingHelpRenderContext<'_>) {
    let StagingHelpRenderContext {
        focus,
        is_merging,
        area,
        theme,
    } = ctx;

    let abort_merge_text = if is_merging { "  A:abort merge" } else { "" };

    let help_text = match focus {
        StagingFocus::Unstaged => {
            let amend_text = if is_merging { "" } else { "  A:amend" };
            format!("j/k:nav  Espace:diff  s/Enter:stage  S:stash fichier  Ctrl+S:stash non stages  a:stage all  d:discard  Tab:→Staged  c:commit{}  P:push  Ctrl+P:force push{}  1:graph  q:quit", amend_text, abort_merge_text)
        }
        StagingFocus::Staged => {
            let amend_text = if is_merging { "" } else { "  A:amend" };
            format!("j/k:nav  Espace:diff  u/Enter:unstage  U:unstage all  Tab:→Unstaged  c:commit{}  P:push  Ctrl+P:force push{}  1:graph  q:quit", amend_text, abort_merge_text)
        }
        StagingFocus::Diff => {
            let amend_text = if is_merging { "" } else { "  A:amend" };
            format!("j/k:scroll  h/l:horizontal  v:vue  Tab/Esc:retour liste  c:commit{}  P:push  Ctrl+P:force push{}  1:graph  q:quit", amend_text, abort_merge_text)
        }
        StagingFocus::CommitMessage => "Enter:confirmer  Esc:annuler  ←→:curseur".to_string(),
    };

    let line = Line::from(vec![Span::styled(
        format!(" {} ", help_text),
        Style::default().fg(theme.text_secondary),
    )]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme.background)),
        area,
    );
}
