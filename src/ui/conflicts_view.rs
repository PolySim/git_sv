//! Vue de résolution de conflits (style GitKraken).

#![allow(dead_code)]

use crate::ui::common::centered_rect;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::git::conflict::{ConflictResolution, ConflictResolutionMode, ConflictType};
use crate::state::{ConflictPanelFocus, ConflictsState};
use crate::ui::theme::current_theme;

pub struct ConflictsRenderContext<'a> {
    pub state: &'a mut ConflictsState,
    pub current_branch: Option<&'a str>,
    pub repo_path: &'a str,
    pub flash_message: Option<&'a str>,
}

pub struct ConflictsHelpOverlayRenderContext {
    pub area: Rect,
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

/// Rend la vue de résolution de conflits.
pub fn render(frame: &mut Frame, ctx: ConflictsRenderContext<'_>) {
    let ConflictsRenderContext {
        state,
        current_branch,
        repo_path,
        flash_message,
    } = ctx;

    let area = frame.area();

    // Layout principal avec status bar en haut
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(10),   // Contenu principal
            Constraint::Length(1), // Help bar
        ])
        .split(area);

    // Status bar
    let status_bar = build_status_bar(state, current_branch, repo_path, flash_message);
    frame.render_widget(status_bar, main_layout[0]);

    // Zone principale en deux panneaux
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(main_layout[1]);

    // Panneau gauche: liste des fichiers
    render_files_panel(frame, state, content_layout[0]);

    // Panneau droit: trois sous-panneaux (Ours, Theirs, Résultat)
    let resolution_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(content_layout[1]);

    // Sauvegarder les hauteurs des panneaux pour le scroll automatique (hauteur - 2 bordures)
    state.ours_panel_height = (resolution_layout[0].height as usize).saturating_sub(2);
    state.theirs_panel_height = (resolution_layout[1].height as usize).saturating_sub(2);
    state.result_panel_height = (resolution_layout[2].height as usize).saturating_sub(2);

    render_ours_panel(frame, state, resolution_layout[0]);
    render_theirs_panel(frame, state, resolution_layout[1]);
    render_result_panel(frame, state, resolution_layout[2]);

    // Help bar
    let help_bar = build_help_bar(state);
    frame.render_widget(help_bar, main_layout[2]);
}

/// Construit la status bar.
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

/// Construit la help bar avec indication du mode actif.
fn build_help_bar<'a>(state: &'a ConflictsState) -> Paragraph<'a> {
    let theme = current_theme();
    let mode_indicator = match state.resolution_mode {
        ConflictResolutionMode::File => "Mode:Fichier",
        ConflictResolutionMode::Block => "Mode:Bloc",
        ConflictResolutionMode::Line => "Mode:Ligne",
    };

    // Aide contextuelle selon le panneau actif et le mode
    let help_text = if state.is_editing {
        // Mode édition : raccourcis d'édition
        "Esc:Annuler  Ctrl+S:Sauvegarder  ↑↓←→:Curseur  Enter:Nouvelle ligne  Backspace:Suppr"
            .to_string()
    } else if state.panel_focus == ConflictPanelFocus::FileList {
        format!(
            "o/←:Ours  t/→:Theirs  Tab:Panneau  ↑↓:Nav  r:Résoudre  V:Finaliser  q:Quitter  A:Avorter | {}",
            mode_indicator
        )
    } else {
        // Aide contextuelle selon le mode de résolution
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

/// Rend le panneau de liste des fichiers.
fn render_files_panel(frame: &mut Frame, state: &ConflictsState, area: Rect) {
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

            // Icône selon le type de conflit
            let type_icon = match file.conflict_type {
                Some(ConflictType::DeletedByUs) => "D←",
                Some(ConflictType::DeletedByThem) => "D→",
                Some(ConflictType::BothAdded) => "A+",
                Some(ConflictType::BothModified) | None => "  ",
            };

            // Déterminer la résolution choisie pour l'affichage
            let resolution_label = if file.is_resolved {
                // Prendre la résolution de la première section (toutes identiques après résolution rapide)
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

/// Rend le panneau Ours.
fn render_ours_panel(frame: &mut Frame, state: &ConflictsState, area: Rect) {
    use crate::git::conflict::ConflictResolutionMode;

    let theme = current_theme();
    let is_focused = state.panel_focus == ConflictPanelFocus::OursPanel;
    let is_file_mode = state.resolution_mode == ConflictResolutionMode::File;
    let is_line_mode = state.resolution_mode == ConflictResolutionMode::Line;
    let title_style = panel_title_style(is_focused);

    // En mode Fichier ou Ligne, ajouter une indication dans le titre
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

    // Construire le contenu
    let mut lines: Vec<Line> = Vec::new();

    for (idx, section) in current_file.conflicts.iter().enumerate() {
        // En mode Fichier, toutes les sections sont considérées comme "sélectionnées"
        let is_selected = if is_file_mode {
            true
        } else {
            idx == state.section_selected
        };

        // Séparateur entre sections
        if idx > 0 {
            lines.push(conflict_separator(area.width));
        }

        // Titre de la section (en mode Fichier, pas de numérotation de section)
        if !is_file_mode {
            lines.push(conflict_section_title(
                idx,
                current_file.conflicts.len(),
                is_selected,
            ));
        }

        // Lignes de contexte avant
        push_context_lines(&mut lines, &section.context_before);

        // Contenu ours avec highlight si sélectionné
        // En mode Ligne, afficher les indicateurs [x] ou [ ]
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
            // Mode Block ou File - affichage standard
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

        // Lignes de contexte après
        push_context_lines(&mut lines, &section.context_after);
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((state.ours_scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

/// Rend le panneau Theirs.
fn render_theirs_panel(frame: &mut Frame, state: &ConflictsState, area: Rect) {
    use crate::git::conflict::ConflictResolutionMode;

    let theme = current_theme();
    let is_focused = state.panel_focus == ConflictPanelFocus::TheirsPanel;
    let is_file_mode = state.resolution_mode == ConflictResolutionMode::File;
    let is_line_mode = state.resolution_mode == ConflictResolutionMode::Line;
    let title_style = panel_title_style(is_focused);

    // En mode Fichier ou Ligne, ajouter une indication dans le titre
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

    // Construire le contenu
    let mut lines: Vec<Line> = Vec::new();

    for (idx, section) in current_file.conflicts.iter().enumerate() {
        // En mode Fichier, toutes les sections sont considérées comme "sélectionnées"
        let is_selected = if is_file_mode {
            true
        } else {
            idx == state.section_selected
        };

        // Séparateur entre sections
        if idx > 0 {
            lines.push(conflict_separator(area.width));
        }

        // Titre de la section (en mode Fichier, pas de numérotation de section)
        if !is_file_mode {
            lines.push(conflict_section_title(
                idx,
                current_file.conflicts.len(),
                is_selected,
            ));
        }

        // Lignes de contexte avant
        push_context_lines(&mut lines, &section.context_before);

        // Contenu theirs avec highlight si sélectionné
        // En mode Ligne, afficher les indicateurs [x] ou [ ]
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
            // Mode Block ou File - affichage standard
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

        // Lignes de contexte après
        push_context_lines(&mut lines, &section.context_after);
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((state.theirs_scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

/// Rend une ligne en mode édition avec le curseur visible.
fn render_edit_line_with_cursor<'a>(line: &'a str, cursor_col: usize, line_num: &str) -> Line<'a> {
    let theme = current_theme();
    let mut spans = Vec::new();

    // Numéro de ligne
    spans.push(Span::styled(
        line_num.to_string(),
        Style::default().fg(theme.text_secondary),
    ));
    spans.push(Span::raw(" "));

    let chars: Vec<char> = line.chars().collect();

    if cursor_col >= chars.len() {
        // Curseur en fin de ligne : tout le texte normal + espace inversé
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
        // Texte avant le curseur
        if cursor_col > 0 {
            let before: String = chars[..cursor_col].iter().collect();
            spans.push(Span::styled(before, Style::default().fg(theme.text_normal)));
        }

        // Caractère sous le curseur (inversé)
        let cursor_char = chars[cursor_col].to_string();
        spans.push(Span::styled(
            cursor_char,
            Style::default()
                .bg(theme.selection_fg)
                .fg(theme.selection_bg),
        ));

        // Texte après le curseur
        if cursor_col + 1 < chars.len() {
            let after: String = chars[cursor_col + 1..].iter().collect();
            spans.push(Span::styled(after, Style::default().fg(theme.text_normal)));
        }
    }

    Line::from(spans)
}

/// Rend le panneau Résultat avec background coloré.
fn render_result_panel(frame: &mut Frame, state: &ConflictsState, area: Rect) {
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

    // En mode édition, afficher le buffer éditable avec curseur et numéros de ligne
    let lines: Vec<Line> = if state.is_editing {
        state
            .edit_buffer
            .iter()
            .enumerate()
            .map(|(idx, content)| {
                let is_cursor_line = idx == state.edit_cursor_line;
                let line_num = format!("{:>3} │", idx + 1);

                if is_cursor_line {
                    // Afficher la ligne avec le curseur visible à la colonne exacte
                    render_edit_line_with_cursor(content, state.edit_cursor_col, &line_num)
                } else {
                    // Ligne normale avec numéro
                    Line::from(vec![
                        Span::styled(line_num, Style::default().fg(theme.text_secondary)),
                        Span::raw(" "),
                        Span::styled(content.to_string(), Style::default().fg(theme.text_normal)),
                    ])
                }
            })
            .collect()
    } else {
        // Mode normal: afficher le contenu résolu avec les couleurs
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

/// Rend une vue compacte de la barre de navigation pour la status bar.
pub fn render_nav_indicator(has_conflicts: bool) -> Line<'static> {
    use ratatui::text::Span;

    let theme = current_theme();
    let mut spans = vec![
        Span::styled("1:Graph", Style::default().fg(theme.text_secondary)),
        Span::styled(" | ", Style::default().fg(theme.text_secondary)),
        Span::styled("2:Staging", Style::default().fg(theme.text_secondary)),
        Span::styled(" | ", Style::default().fg(theme.text_secondary)),
        Span::styled("3:Branches", Style::default().fg(theme.text_secondary)),
    ];

    if has_conflicts {
        spans.push(Span::styled(
            " | ",
            Style::default().fg(theme.text_secondary),
        ));
        spans.push(Span::styled(
            "4:Conflits",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ));
    }

    Line::from(spans)
}

/// Rend l'overlay d'aide pour la vue conflits.
pub fn render_help_overlay(frame: &mut Frame, ctx: ConflictsHelpOverlayRenderContext) {
    let ConflictsHelpOverlayRenderContext { area } = ctx;

    let theme = current_theme();
    let popup_area = centered_rect(70, 80, area);

    frame.render_widget(Clear, popup_area);

    let content = vec![
        Line::from(vec![Span::styled(
            "Raccourcis de la vue Conflits",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().fg(theme.warning),
        )]),
        Line::from("  ↑/↓ ou j/k  - Naviguer (fichiers / sections / lignes selon le panneau)"),
        Line::from("  Tab         - Panneau suivant (Fichiers → Ours → Theirs → Résultat)"),
        Line::from("  Shift+Tab   - Panneau précédent"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Résolution",
            Style::default().fg(theme.warning),
        )]),
        Line::from("  o           - Garder la version 'ours' (HEAD)"),
        Line::from("  t           - Garder la version 'theirs' (branche mergée)"),
        Line::from("  b           - Garder les deux versions (mode Bloc uniquement)"),
        Line::from("  Espace      - Choisir/déchoisir un bloc ou une ligne"),
        Line::from("  Enter       - Valider et sauvegarder le fichier courant"),
        Line::from("  r           - Marquer comme résolu (depuis la liste de fichiers)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Édition du résultat",
            Style::default().fg(theme.warning),
        )]),
        Line::from("  i ou e      - Entrer en mode édition (panneau Résultat)"),
        Line::from("  Esc         - Quitter le mode édition"),
        Line::from("  ↑/↓/←/→     - Déplacer le curseur"),
        Line::from("  Caractères  - Insérer du texte"),
        Line::from("  Backspace   - Supprimer le caractère avant"),
        Line::from("  Delete      - Supprimer le caractère sous le curseur"),
        Line::from("  Enter       - Insérer une nouvelle ligne"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions globales",
            Style::default().fg(theme.warning),
        )]),
        Line::from("  F/B/L       - Mode Fichier/Bloc/Ligne (touche directe)"),
        Line::from("  V           - Finaliser le merge (créer le commit)"),
        Line::from("  q ou Esc    - Annuler le merge et revenir au graph"),
        Line::from("  1/2/3       - Basculer vers Graph/Staging/Branches"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Appuyez sur ? pour fermer cette aide",
            Style::default().fg(theme.text_secondary),
        )]),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title("Aide - Résolution de conflits")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}
