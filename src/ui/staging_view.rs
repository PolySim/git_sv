//! Rendu de la vue staging (fichiers stagés/non stagés, diff, commit).

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::git::repo::{FileStatusKind, StatusEntry};
use crate::i18n::{text, text_owned};
use crate::state::{StagingFocus, StagingState};
use crate::ui::common::StatusBarConfig;
use crate::ui::image_preview::ImagePreviewState;
use crate::ui::theme::{current_theme, Theme};

pub struct StagingRenderContext<'a> {
    pub staging_state: &'a StagingState,
    pub current_branch: Option<&'a str>,
    pub repo_path: &'a str,
    pub flash_message: Option<&'a str>,
    pub is_merging: bool,
    pub image_state: &'a mut ImagePreviewState,
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
    selection_anchor: Option<usize>,
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
        image_state,
    } = ctx;

    let theme = current_theme();
    let layout = super::staging_layout::build_staging_layout(frame.area());

    // Status bar.
    crate::ui::common::render_status_bar(
        frame,
        StatusBarConfig {
            view_title: text("staging", "staging"),
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
            title: text("Non indexes", "Unstaged"),
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
            title: text("Indexes", "Staged"),
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
            image_state,
        },
    );

    // Zone de message commit.
    render_commit_input(
        frame,
        CommitInputRenderContext {
            message: &staging_state.commit_message,
            cursor_pos: staging_state.cursor_position,
            selection_anchor: staging_state.selection_anchor,
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
            let kind = entry.status_kind();

            let status_icon = match kind {
                FileStatusKind::Staged | FileStatusKind::NewStaged => "●",
                FileStatusKind::Modified => "M",
                FileStatusKind::Deleted => "D",
                FileStatusKind::Untracked => "?",
                FileStatusKind::Renamed => "R",
                FileStatusKind::Conflicted => "!",
            };

            let status_color = match kind {
                FileStatusKind::Staged | FileStatusKind::NewStaged => theme.success,
                FileStatusKind::Modified => theme.warning,
                FileStatusKind::Deleted => theme.error,
                FileStatusKind::Untracked => theme.text_secondary,
                FileStatusKind::Renamed => theme.info,
                FileStatusKind::Conflicted => theme.error,
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
        selection_anchor,
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
        text(
            " Amend commit (Entree pour valider) ",
            " Amend commit (Enter to confirm) ",
        )
    } else if has_staged_files {
        text(
            " Message de commit (Entree pour valider) ",
            " Commit message (Enter to confirm) ",
        )
    } else {
        text(
            " Message de commit (aucun fichier indexe) ",
            " Commit message (no staged files) ",
        )
    };

    let display_text = if message.is_empty() && !is_focused {
        if is_amending {
            text(
                "Appuyez sur 'A' pour amender le dernier commit...",
                "Press 'A' to amend the last commit...",
            )
        } else {
            text(
                "Appuyez sur 'c' pour ecrire un message de commit...",
                "Press 'c' to write a commit message...",
            )
        }
    } else {
        message
    };

    let text_style = if message.is_empty() && !is_focused {
        Style::default().fg(theme.text_secondary)
    } else {
        Style::default().fg(theme.text_normal)
    };

    let rendered_text = if is_focused {
        crate::ui::text_edit::text_with_selection(
            message,
            cursor_pos,
            selection_anchor,
            text_style,
            Style::default()
                .fg(theme.selection_fg)
                .bg(theme.selection_bg),
        )
    } else {
        display_text.into()
    };

    let paragraph = Paragraph::new(rendered_text).style(text_style).block(
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

    let abort_merge_text = if is_merging {
        text("  A:annuler fusion", "  A:abort merge")
    } else {
        ""
    };

    let help_text = match focus {
        StagingFocus::Unstaged => {
            let amend_text = if is_merging {
                ""
            } else {
                text("  A:amender", "  A:amend")
            };
            text_owned(
                format!("j/k:naviguer  Espace:diff  s/Entree:indexer  S:stocker fichier  Ctrl+S:stocker non indexes  a:indexer tout  d:abandonner  Tab:→Indexes  c:commit{}  P:push  Ctrl+P:force push{}  1:graphe  q:quitter", amend_text, abort_merge_text),
                format!("j/k:nav  Space:diff  s/Enter:stage  S:stash file  Ctrl+S:stash unstaged  a:stage all  d:discard  Tab:→Staged  c:commit{}  P:push  Ctrl+P:force push{}  1:graph  q:quit", amend_text, abort_merge_text),
            )
        }
        StagingFocus::Staged => {
            let amend_text = if is_merging {
                ""
            } else {
                text("  A:amender", "  A:amend")
            };
            text_owned(
                format!("j/k:naviguer  Espace:diff  u/Entree:desindexer  U:desindexer tout  Tab:→Non indexes  c:commit{}  P:push  Ctrl+P:force push{}  1:graphe  q:quitter", amend_text, abort_merge_text),
                format!("j/k:nav  Space:diff  u/Enter:unstage  U:unstage all  Tab:→Unstaged  c:commit{}  P:push  Ctrl+P:force push{}  1:graph  q:quit", amend_text, abort_merge_text),
            )
        }
        StagingFocus::Diff => {
            let amend_text = if is_merging {
                ""
            } else {
                text("  A:amender", "  A:amend")
            };
            text_owned(
                format!("j/k:defiler  h/l:horizontal  v:vue  Tab/Echap:retour liste  c:commit{}  P:push  Ctrl+P:force push{}  1:graphe  q:quitter", amend_text, abort_merge_text),
                format!("j/k:scroll  h/l:horizontal  v:view  Tab/Esc:back to list  c:commit{}  P:push  Ctrl+P:force push{}  1:graph  q:quit", amend_text, abort_merge_text),
            )
        }
        StagingFocus::CommitMessage => text_owned(
            "Entree:confirmer  Echap:annuler  ←→:curseur",
            "Enter:confirm  Esc:cancel  ←→:cursor",
        ),
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
