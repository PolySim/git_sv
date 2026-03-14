//! Vue de résolution de conflits (style GitKraken).

mod help;
mod panels;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::git::conflict::ConflictResolutionMode;
use crate::state::{ConflictPanelFocus, ConflictsState};
use crate::ui::theme::current_theme;

pub use help::{render_help_overlay, ConflictsHelpOverlayRenderContext};
use panels::{render_files_panel, render_ours_panel, render_result_panel, render_theirs_panel};

pub struct ConflictsRenderContext<'a> {
    pub state: &'a mut ConflictsState,
    pub current_branch: Option<&'a str>,
    pub repo_path: &'a str,
    pub flash_message: Option<&'a str>,
}

fn panel_title_style(is_focused: bool) -> Style {
    let theme = current_theme();

    if is_focused {
        Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.text_normal)
            .add_modifier(Modifier::BOLD)
    }
}

fn panel_border_style(is_focused: bool, border_color: ratatui::style::Color) -> Style {
    let theme = current_theme();
    Style::default().fg(if is_focused {
        border_color
    } else {
        theme.border_inactive
    })
}

fn render_empty_panel(
    frame: &mut Frame,
    block: Block<'static>,
    message: &'static str,
    style: Style,
    area: Rect,
) {
    let empty = Paragraph::new(message).block(block).style(style);
    frame.render_widget(empty, area);
}

fn conflict_separator(width: u16) -> Line<'static> {
    let theme = current_theme();
    Line::from(vec![Span::styled(
        "─".repeat(width.saturating_sub(2) as usize),
        Style::default().fg(theme.text_secondary),
    )])
}

fn conflict_section_title(idx: usize, total: usize, is_selected: bool) -> Line<'static> {
    let theme = current_theme();
    Line::from(vec![Span::styled(
        format!("#{}/{}", idx + 1, total),
        Style::default()
            .fg(if is_selected {
                theme.warning
            } else {
                theme.text_secondary
            })
            .add_modifier(Modifier::BOLD),
    )])
}

fn push_context_lines(lines: &mut Vec<Line<'static>>, context_lines: &[String]) {
    let theme = current_theme();
    lines.extend(context_lines.iter().map(|line| {
        Line::from(vec![Span::styled(
            format!("  {}", line),
            Style::default().fg(theme.text_secondary),
        )])
    }));
}

fn render_edit_line_with_cursor<'a>(line: &'a str, cursor_col: usize, line_num: &str) -> Line<'a> {
    let theme = current_theme();
    let mut spans = Vec::new();

    spans.push(Span::styled(
        line_num.to_string(),
        Style::default().fg(theme.text_secondary),
    ));
    spans.push(Span::raw(" "));

    let chars: Vec<char> = line.chars().collect();

    if cursor_col >= chars.len() {
        spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(theme.text_normal),
        ));
        spans.push(Span::styled(
            " ",
            Style::default()
                .bg(theme.selection_fg)
                .fg(theme.selection_bg),
        ));
    } else {
        if cursor_col > 0 {
            let before: String = chars[..cursor_col].iter().collect();
            spans.push(Span::styled(before, Style::default().fg(theme.text_normal)));
        }

        let cursor_char = chars[cursor_col].to_string();
        spans.push(Span::styled(
            cursor_char,
            Style::default()
                .bg(theme.selection_fg)
                .fg(theme.selection_bg),
        ));

        if cursor_col + 1 < chars.len() {
            let after: String = chars[cursor_col + 1..].iter().collect();
            spans.push(Span::styled(after, Style::default().fg(theme.text_normal)));
        }
    }

    Line::from(spans)
}

/// Rend la vue de résolution de conflits.
pub fn render(frame: &mut Frame, ctx: ConflictsRenderContext<'_>) {
    let ConflictsRenderContext {
        state,
        current_branch,
        repo_path,
        flash_message,
    } = ctx;

    let area = frame.area();

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    let status_bar = build_status_bar(state, current_branch, repo_path, flash_message);
    frame.render_widget(status_bar, main_layout[0]);

    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(main_layout[1]);

    render_files_panel(frame, state, content_layout[0]);

    let resolution_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(content_layout[1]);

    state.ours_panel_height = (resolution_layout[0].height as usize).saturating_sub(2);
    state.theirs_panel_height = (resolution_layout[1].height as usize).saturating_sub(2);
    state.result_panel_height = (resolution_layout[2].height as usize).saturating_sub(2);

    render_ours_panel(frame, state, resolution_layout[0]);
    render_theirs_panel(frame, state, resolution_layout[1]);
    render_result_panel(frame, state, resolution_layout[2]);

    let help_bar = build_help_bar(state);
    frame.render_widget(help_bar, main_layout[2]);
}

fn build_status_bar<'a>(
    state: &'a ConflictsState,
    current_branch: Option<&'a str>,
    repo_path: &'a str,
    flash_message: Option<&'a str>,
) -> Paragraph<'a> {
    let theme = current_theme();
    let branch_str = current_branch.unwrap_or("HEAD détachée");
    let repo_name = std::path::Path::new(repo_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(repo_path);

    let unresolved_count = state.all_files.iter().filter(|f| !f.is_resolved).count();

    let status_text = if let Some(msg) = flash_message {
        format!(
            "{} · {} · {} · {}",
            repo_name, branch_str, state.operation_description, msg
        )
    } else {
        format!(
            "{} · {} · {} · {} fichier(s) non résolu(s)",
            repo_name, branch_str, state.operation_description, unresolved_count
        )
    };

    Paragraph::new(status_text)
        .style(
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
}

fn build_help_bar<'a>(state: &'a ConflictsState) -> Paragraph<'a> {
    let theme = current_theme();
    let mode_indicator = match state.resolution_mode {
        ConflictResolutionMode::File => "Mode:Fichier",
        ConflictResolutionMode::Block => "Mode:Bloc",
        ConflictResolutionMode::Line => "Mode:Ligne",
    };

    let help_text = if state.is_editing {
        "Esc:Annuler  Ctrl+S:Sauvegarder  ↑↓←→:Curseur  Enter:Nouvelle ligne  Backspace:Suppr"
            .to_string()
    } else if state.panel_focus == ConflictPanelFocus::FileList {
        format!(
            "o/←:Ours  t/→:Theirs  Tab:Panneau  ↑↓:Nav  r:Résoudre  V:Finaliser  q:Quitter  A:Avorter | {}",
            mode_indicator
        )
    } else {
        let action_help = match state.resolution_mode {
            ConflictResolutionMode::File => "Enter:Choisir  o:Ours  t:Theirs  Tab:Panneau  ↑↓:Nav",
            ConflictResolutionMode::Block => {
                "Espace:Choisir  Enter:Valider  b:Les deux  Tab:Panneau  ↑↓:Nav"
            }
            ConflictResolutionMode::Line => {
                "Espace:Choisir ligne  Enter:Valider  Tab:Panneau  ↑↓:Nav"
            }
        };
        format!(
            "{}  F/B/L:Mode  i:Éditer  V:Finaliser  q:Quitter  A:Avorter | {}",
            action_help, mode_indicator
        )
    };

    Paragraph::new(help_text)
        .style(Style::default().fg(theme.text_secondary))
        .alignment(Alignment::Center)
}
