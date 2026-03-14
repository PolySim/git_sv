use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::git::conflict::{ConflictResolution, ConflictResolutionMode, ConflictType};
use crate::state::{ConflictPanelFocus, ConflictsState};
use crate::ui::theme::current_theme;

use super::{
    conflict_section_title, conflict_separator, panel_border_style, panel_title_style,
    push_context_lines, render_edit_line_with_cursor, render_empty_panel,
};

pub(super) fn render_files_panel(
    frame: &mut Frame,
    state: &ConflictsState,
    area: ratatui::layout::Rect,
) {
    let theme = current_theme();
    let is_focused = state.panel_focus == ConflictPanelFocus::FileList;
    let title_style = panel_title_style(is_focused);

    let block = Block::default()
        .title(Span::styled("Fichiers en conflit", title_style))
        .borders(Borders::ALL)
        .border_style(panel_border_style(is_focused, theme.warning));

    if state.all_files.is_empty() {
        render_empty_panel(
            frame,
            block,
            "Aucun fichier en conflit",
            Style::default().fg(theme.text_secondary),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = state
        .all_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let status_icon = if file.is_resolved { "✓" } else { "✗" };
            let type_icon = match file.conflict_type {
                Some(ConflictType::DeletedByUs) => "D←",
                Some(ConflictType::DeletedByThem) => "D→",
                Some(ConflictType::BothAdded) => "A+",
                Some(ConflictType::BothModified) | None => "  ",
            };

            let resolution_label = if file.is_resolved {
                file.conflicts
                    .first()
                    .and_then(|s| s.resolution)
                    .map_or(String::new(), |r| match r {
                        ConflictResolution::Ours => " [Ours]".to_string(),
                        ConflictResolution::Theirs => " [Theirs]".to_string(),
                        ConflictResolution::Both => " [Les deux]".to_string(),
                    })
            } else {
                String::new()
            };

            let color = if file.is_resolved {
                theme.success
            } else {
                theme.error
            };

            let style = if idx == state.file_selected {
                Style::default()
                    .fg(color)
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            let label = format!(
                "{} {}{}{}",
                status_icon, type_icon, file.path, resolution_label
            );
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

pub(super) fn render_ours_panel(
    frame: &mut Frame,
    state: &ConflictsState,
    area: ratatui::layout::Rect,
) {
    let theme = current_theme();
    let is_focused = state.panel_focus == ConflictPanelFocus::OursPanel;
    let is_file_mode = state.resolution_mode == ConflictResolutionMode::File;
    let is_line_mode = state.resolution_mode == ConflictResolutionMode::Line;
    let title_style = panel_title_style(is_focused);

    let title_text = if is_file_mode {
        format!(" {} [Fichier entier] ", state.ours_branch_name)
    } else if is_line_mode {
        format!(" {} [Mode Ligne] ", state.ours_branch_name)
    } else {
        format!(" {} ", state.ours_branch_name)
    };

    let border_color = if is_focused {
        if is_file_mode {
            theme.success
        } else if is_line_mode {
            theme.info
        } else {
            theme.warning
        }
    } else {
        theme.border_inactive
    };

    let block = Block::default()
        .title(Span::styled(title_text, title_style))
        .borders(Borders::ALL)
        .border_style(panel_border_style(is_focused, border_color));

    let Some(current_file) = state.all_files.get(state.file_selected) else {
        render_empty_panel(
            frame,
            block,
            "Sélectionnez un fichier",
            Style::default().fg(theme.text_secondary),
            area,
        );
        return;
    };

    if current_file.conflicts.is_empty() {
        render_empty_panel(
            frame,
            block,
            "Aucun conflit",
            Style::default().fg(theme.success),
            area,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for (idx, section) in current_file.conflicts.iter().enumerate() {
        let is_selected = if is_file_mode {
            true
        } else {
            idx == state.section_selected
        };

        if idx > 0 {
            lines.push(conflict_separator(area.width));
        }
        if !is_file_mode {
            lines.push(conflict_section_title(
                idx,
                current_file.conflicts.len(),
                is_selected,
            ));
        }

        push_context_lines(&mut lines, &section.context_before);

        if is_line_mode && is_selected {
            for (line_idx, line) in section.ours.iter().enumerate() {
                let is_current_line = line_idx == state.line_selected && is_focused;
                let is_included = section
                    .line_level_resolution
                    .as_ref()
                    .map(|lr| {
                        lr.ours_lines_included
                            .get(line_idx)
                            .copied()
                            .unwrap_or(false)
                    })
                    .unwrap_or(true);

                let indicator = if is_included { "[x]" } else { "[ ]" };
                let style = if is_current_line {
                    Style::default()
                        .fg(if is_included {
                            theme.success
                        } else {
                            theme.text_secondary
                        })
                        .bg(theme.selection_bg)
                        .add_modifier(Modifier::BOLD)
                } else if is_included {
                    Style::default().fg(theme.success)
                } else {
                    Style::default().fg(theme.text_secondary)
                };

                lines.push(Line::from(vec![Span::styled(
                    format!("{} {}", indicator, line),
                    style,
                )]));
            }
        } else {
            let ours_style = if is_selected
                && matches!(
                    section.resolution,
                    Some(ConflictResolution::Ours | ConflictResolution::Both)
                ) {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.success)
            };

            for line in &section.ours {
                lines.push(Line::from(vec![Span::styled(
                    format!("> {}", line),
                    ours_style,
                )]));
            }
        }

        push_context_lines(&mut lines, &section.context_after);
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((state.ours_scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

pub(super) fn render_theirs_panel(
    frame: &mut Frame,
    state: &ConflictsState,
    area: ratatui::layout::Rect,
) {
    let theme = current_theme();
    let is_focused = state.panel_focus == ConflictPanelFocus::TheirsPanel;
    let is_file_mode = state.resolution_mode == ConflictResolutionMode::File;
    let is_line_mode = state.resolution_mode == ConflictResolutionMode::Line;
    let title_style = panel_title_style(is_focused);

    let title_text = if is_file_mode {
        format!(" {} [Fichier entier] ", state.theirs_branch_name)
    } else if is_line_mode {
        format!(" {} [Mode Ligne] ", state.theirs_branch_name)
    } else {
        format!(" {} ", state.theirs_branch_name)
    };

    let border_color = if is_focused {
        if is_file_mode {
            theme.success
        } else if is_line_mode {
            theme.info
        } else {
            theme.warning
        }
    } else {
        theme.border_inactive
    };

    let block = Block::default()
        .title(Span::styled(title_text, title_style))
        .borders(Borders::ALL)
        .border_style(panel_border_style(is_focused, border_color));

    let Some(current_file) = state.all_files.get(state.file_selected) else {
        render_empty_panel(
            frame,
            block,
            "Sélectionnez un fichier",
            Style::default().fg(theme.text_secondary),
            area,
        );
        return;
    };

    if current_file.conflicts.is_empty() {
        render_empty_panel(
            frame,
            block,
            "Aucun conflit",
            Style::default().fg(theme.success),
            area,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for (idx, section) in current_file.conflicts.iter().enumerate() {
        let is_selected = if is_file_mode {
            true
        } else {
            idx == state.section_selected
        };

        if idx > 0 {
            lines.push(conflict_separator(area.width));
        }
        if !is_file_mode {
            lines.push(conflict_section_title(
                idx,
                current_file.conflicts.len(),
                is_selected,
            ));
        }

        push_context_lines(&mut lines, &section.context_before);

        if is_line_mode && is_selected {
            for (line_idx, line) in section.theirs.iter().enumerate() {
                let is_current_line = line_idx == state.line_selected && is_focused;
                let is_included = section
                    .line_level_resolution
                    .as_ref()
                    .map(|lr| {
                        lr.theirs_lines_included
                            .get(line_idx)
                            .copied()
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);

                let indicator = if is_included { "[x]" } else { "[ ]" };
                let style = if is_current_line {
                    Style::default()
                        .fg(if is_included {
                            theme.info
                        } else {
                            theme.text_secondary
                        })
                        .bg(theme.selection_bg)
                        .add_modifier(Modifier::BOLD)
                } else if is_included {
                    Style::default().fg(theme.info)
                } else {
                    Style::default().fg(theme.text_secondary)
                };

                lines.push(Line::from(vec![Span::styled(
                    format!("{} {}", indicator, line),
                    style,
                )]));
            }
        } else {
            let theirs_style = if is_selected
                && matches!(
                    section.resolution,
                    Some(ConflictResolution::Theirs | ConflictResolution::Both)
                ) {
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.info)
            };

            for line in &section.theirs {
                lines.push(Line::from(vec![Span::styled(
                    format!("> {}", line),
                    theirs_style,
                )]));
            }
        }

        push_context_lines(&mut lines, &section.context_after);
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((state.theirs_scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

pub(super) fn render_result_panel(
    frame: &mut Frame,
    state: &ConflictsState,
    area: ratatui::layout::Rect,
) {
    use crate::git::conflict::{generate_resolved_content_with_source, LineSource};

    let theme = current_theme();
    let is_focused = state.panel_focus == ConflictPanelFocus::ResultPanel;
    let title_text = if state.is_editing {
        "Résultat [ÉDITION]"
    } else {
        "Résultat"
    };
    let title_style = if state.is_editing {
        Style::default()
            .fg(theme.secondary)
            .add_modifier(Modifier::BOLD)
    } else if is_focused {
        Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.text_normal)
            .add_modifier(Modifier::BOLD)
    };

    let block = Block::default()
        .title(Span::styled(title_text, title_style))
        .borders(Borders::ALL)
        .border_style(panel_border_style(
            state.is_editing || is_focused,
            if state.is_editing {
                theme.secondary
            } else {
                theme.warning
            },
        ));

    let Some(current_file) = state.all_files.get(state.file_selected) else {
        render_empty_panel(
            frame,
            block,
            "Sélectionnez un fichier",
            Style::default().fg(theme.text_secondary),
            area,
        );
        return;
    };

    let lines: Vec<Line> = if state.is_editing {
        state
            .edit_buffer
            .iter()
            .enumerate()
            .map(|(idx, content)| {
                let is_cursor_line = idx == state.edit_cursor_line;
                let line_num = format!("{:>3} │", idx + 1);

                if is_cursor_line {
                    render_edit_line_with_cursor(content, state.edit_cursor_col, &line_num)
                } else {
                    Line::from(vec![
                        Span::styled(line_num, Style::default().fg(theme.text_secondary)),
                        Span::raw(" "),
                        Span::styled(content.to_string(), Style::default().fg(theme.text_normal)),
                    ])
                }
            })
            .collect()
    } else {
        let resolved = generate_resolved_content_with_source(current_file, state.resolution_mode);

        resolved
            .into_iter()
            .map(|rline| {
                let style = match rline.source {
                    LineSource::Context => Style::default().fg(theme.text_normal),
                    LineSource::Ours => Style::default().bg(theme.ours_bg).fg(theme.text_normal),
                    LineSource::Theirs => {
                        Style::default().bg(theme.theirs_bg).fg(theme.text_normal)
                    }
                    LineSource::ConflictMarker => Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                };
                Line::from(vec![Span::styled(rline.content, style)])
            })
            .collect()
    };

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((state.result_scroll as u16, 0));

    frame.render_widget(paragraph, area);
}
