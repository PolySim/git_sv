//! Vue de résolution de conflits (style GitKraken) - Refonte complète.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::git::conflict::{ConflictResolution, ConflictResolutionMode, MergeFile};
use crate::state::{ConflictPanelFocus, ConflictsState};

/// Rend la vue de résolution de conflits.
pub fn render(
    frame: &mut Frame,
    state: &ConflictsState,
    current_branch: &Option<String>,
    repo_path: &str,
    flash_message: Option<&str>,
) {
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

    let right_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(content_layout[1]);

    render_ours_theirs_panels(frame, state, right_panel[0]);
    render_result_panel(frame, state, right_panel[1]);

    let help_bar = build_help_bar(state);
    frame.render_widget(help_bar, main_layout[2]);
}

fn build_status_bar<'a>(
    state: &'a ConflictsState,
    current_branch: &'a Option<String>,
    repo_path: &'a str,
    flash_message: Option<&'a str>,
) -> Paragraph<'a> {
    let branch_str = current_branch.as_deref().unwrap_or("HEAD détachée");
    let repo_name = std::path::Path::new(repo_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(repo_path);

    let unresolved_count = state
        .all_files
        .iter()
        .filter(|f| f.has_conflicts && !f.is_resolved)
        .count();

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
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
}

fn build_help_bar<'a>(state: &'a ConflictsState) -> Paragraph<'a> {
    let mode_str = match state.resolution_mode {
        ConflictResolutionMode::File => "fichier",
        ConflictResolutionMode::Block => "bloc",
        ConflictResolutionMode::Line => "ligne",
    };

    let help_text = format!(
        "Mode: {}  o:ours  t:theirs  b:both  j/k:naviguer  Tab:panneau  F:{}  Enter:valider  V:finaliser  q:abort",
        mode_str,
        match state.resolution_mode {
            ConflictResolutionMode::File => "bloc",
            ConflictResolutionMode::Block => "ligne",
            ConflictResolutionMode::Line => "fichier",
        }
    );

    Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
}

fn render_files_panel(frame: &mut Frame, state: &ConflictsState, area: Rect) {
    let block = Block::default()
        .title("Fichiers")
        .borders(Borders::ALL)
        .border_style(if state.panel_focus == ConflictPanelFocus::FileList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        });

    if state.all_files.is_empty() {
        let empty = Paragraph::new("Aucun fichier dans le merge")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .all_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let (icon, color) = if !file.has_conflicts {
                ("✓ ", Color::Green)
            } else if file.is_resolved {
                ("◉ ", Color::Yellow)
            } else {
                ("✗ ", Color::Red)
            };

            let style = if idx == state.file_selected {
                Style::default()
                    .fg(color)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            let label = format!("{}{}", icon, file.path);
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_ours_theirs_panels(frame: &mut Frame, state: &ConflictsState, area: Rect) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let ours_block = Block::default()
        .title("Ours (HEAD)")
        .borders(Borders::ALL)
        .border_style(if state.panel_focus == ConflictPanelFocus::OursPanel {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        });

    let theirs_block = Block::default()
        .title("Theirs")
        .borders(Borders::ALL)
        .border_style(if state.panel_focus == ConflictPanelFocus::TheirsPanel {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Blue)
        });

    let current_file = state.all_files.get(state.file_selected);

    let ours_content = if let Some(file) = current_file {
        if let Some(section) = file.conflicts.get(state.section_selected) {
            build_ours_content(section, state)
        } else {
            vec![Line::from("")]
        }
    } else {
        vec![Line::from("Sélectionnez un fichier")]
    };

    let theirs_content = if let Some(file) = current_file {
        if let Some(section) = file.conflicts.get(state.section_selected) {
            build_theirs_content(section, state)
        } else {
            vec![Line::from("")]
        }
    } else {
        vec![Line::from("Sélectionnez un fichier")]
    };

    let ours_paragraph = Paragraph::new(ours_content)
        .block(ours_block)
        .wrap(Wrap { trim: true });
    let theirs_paragraph = Paragraph::new(theirs_content)
        .block(theirs_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(ours_paragraph, split[0]);
    frame.render_widget(theirs_paragraph, split[1]);
}

fn build_ours_content<'a>(
    section: &'a crate::git::conflict::ConflictSection,
    state: &'a ConflictsState,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    if !section.context_before.is_empty() {
        for line in &section.context_before {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let is_selected_section = state.line_selected == 0
        && matches!(
            state.panel_focus,
            ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel
        );

    if state.resolution_mode == ConflictResolutionMode::Line {
        for (idx, line) in section.ours.iter().enumerate() {
            let is_line_selected = state.line_selected == idx + 1;
            let style = if is_line_selected {
                Style::default()
                    .fg(Color::Green)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            lines.push(Line::from(Span::styled(format!("> {}", line), style)));
        }
    } else {
        if let Some(resolution) = &section.resolution {
            match resolution {
                ConflictResolution::Ours => {
                    lines.push(Line::from(Span::styled(
                        "◄─ Sélectionné",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                ConflictResolution::Both => {
                    lines.push(Line::from(Span::styled(
                        "◄─ Both ─►",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                ConflictResolution::Theirs => {}
            }
        } else if is_selected_section {
            lines.push(Line::from(Span::styled(
                "<<<<<<< Conflit",
                Style::default().fg(Color::Yellow),
            )));
        }

        for line in &section.ours {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::Green),
            )));
        }
    }

    if !section.context_after.is_empty() {
        for line in &section.context_after {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines
}

fn build_theirs_content<'a>(
    section: &'a crate::git::conflict::ConflictSection,
    state: &'a ConflictsState,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    if !section.context_before.is_empty() {
        for line in &section.context_before {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let is_selected_section = state.line_selected == 0
        && matches!(
            state.panel_focus,
            ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel
        );

    if state.resolution_mode == ConflictResolutionMode::Line {
        for (idx, line) in section.theirs.iter().enumerate() {
            let is_line_selected = state.line_selected == idx + 1;
            let style = if is_line_selected {
                Style::default()
                    .fg(Color::Blue)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Blue)
            };
            lines.push(Line::from(Span::styled(format!("> {}", line), style)));
        }
    } else {
        if let Some(resolution) = &section.resolution {
            match resolution {
                ConflictResolution::Theirs => {
                    lines.push(Line::from(Span::styled(
                        "─► Sélectionné",
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                ConflictResolution::Both => {
                    // Already shown in ours
                }
                ConflictResolution::Ours => {}
            }
        } else if is_selected_section {
            lines.push(Line::from(Span::styled(
                "======= Conflit",
                Style::default().fg(Color::Yellow),
            )));
        }

        for line in &section.theirs {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::Blue),
            )));
        }

        if !section.resolution.is_some() && is_selected_section {
            lines.push(Line::from(Span::styled(
                ">>>>>>>",
                Style::default().fg(Color::Yellow),
            )));
        }
    }

    if !section.context_after.is_empty() {
        for line in &section.context_after {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines
}

fn render_result_panel(frame: &mut Frame, state: &ConflictsState, area: Rect) {
    let block = Block::default()
        .title("Résultat")
        .borders(Borders::ALL)
        .border_style(if state.panel_focus == ConflictPanelFocus::ResultPanel {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        });

    let current_file = state.all_files.get(state.file_selected);

    let content = if let Some(file) = current_file {
        if file.conflicts.is_empty() {
            vec![Line::from(Span::styled(
                "Fichier sans conflit",
                Style::default().fg(Color::Green),
            ))]
        } else {
            build_result_content(file, state)
        }
    } else {
        vec![Line::from("Sélectionnez un fichier")]
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn build_result_content<'a>(file: &'a MergeFile, state: &'a ConflictsState) -> Vec<Line<'a>> {
    use crate::git::conflict::generate_resolved_content;

    let resolved = generate_resolved_content(file, state.resolution_mode);

    if resolved.is_empty() {
        return vec![Line::from(Span::styled(
            "Résultat en attente...",
            Style::default().fg(Color::DarkGray),
        ))];
    }

    resolved
        .iter()
        .map(|line| {
            let is_conflict = line.starts_with("<<<<<<<")
                || line.starts_with("=======")
                || line.starts_with(">>>>>>>");
            let style = if is_conflict {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(line.clone(), style))
        })
        .collect()
}

pub fn render_nav_indicator(has_conflicts: bool) -> Line<'static> {
    use ratatui::text::Span;

    let mut spans = vec![
        Span::styled("1:Graph", Style::default().fg(Color::DarkGray)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled("2:Staging", Style::default().fg(Color::DarkGray)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled("3:Branches", Style::default().fg(Color::DarkGray)),
    ];

    if has_conflicts {
        spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            "4:Conflits",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }

    Line::from(spans)
}

pub fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(70, 80, area);

    frame.render_widget(Clear, popup_area);

    let content = vec![
        Line::from(vec![Span::styled(
            "Raccourcis de la vue Conflits",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  j/k ou ↑/↓  - Naviguer entre les sections/lignes"),
        Line::from("  Tab         - Fichier suivant"),
        Line::from("  Shift+Tab   - Panneau suivant (Files → Ours → Theirs → Result)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Résolution",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  o           - Garder la version 'ours' (HEAD)"),
        Line::from("  t           - Garder la version 'theirs' (branche mergée)"),
        Line::from("  b           - Garder les deux versions"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Modes de résolution",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  F           - Mode fichier entier"),
        Line::from("  B           - Mode bloc (par défaut)"),
        Line::from("  L           - Mode ligne par ligne"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  Enter       - Valider la résolution du fichier courant"),
        Line::from("  V           - Finaliser le merge (quand tout est résolu)"),
        Line::from("  q ou Esc    - Annuler le merge"),
        Line::from("  1/2/3       - Basculer vers Graph/Staging/Branches"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Appuyez sur ? pour fermer cette aide",
            Style::default().fg(Color::Gray),
        )]),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title("Aide - Résolution de conflits")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
