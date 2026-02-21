//! Vue de résolution de conflits (style GitKraken).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::git::conflict::ConflictResolution;
use crate::state::ConflictsState;

/// Rend la vue de résolution de conflits.
pub fn render(
    frame: &mut Frame,
    state: &ConflictsState,
    current_branch: &Option<String>,
    repo_path: &str,
    flash_message: Option<&str>,
) {
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
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_layout[1]);

    // Panneau gauche: liste des fichiers
    render_files_panel(frame, state, content_layout[0]);

    // Panneau droit: résolution du conflit sélectionné
    render_resolution_panel(frame, state, content_layout[1]);

    // Help bar
    let help_bar = build_help_bar();
    frame.render_widget(help_bar, main_layout[2]);
}

/// Construit la status bar.
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
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
}

/// Construit la help bar.
fn build_help_bar<'a>() -> Paragraph<'a> {
    let help_text = "Tab:panneau  ↑/↓:naviguer  o:ours  t:theirs  b:both  F:mode  Enter:valider  V:finaliser  q:abort  ?:aide";

    Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
}

/// Rend le panneau de liste des fichiers.
fn render_files_panel(frame: &mut Frame, state: &ConflictsState, area: Rect) {
    let block = Block::default()
        .title("Fichiers en conflit")
        .borders(Borders::ALL)
        .border_style(Style::default());

    if state.all_files.is_empty() {
        let empty = Paragraph::new("Aucun fichier en conflit")
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
            let icon = if file.is_resolved { "✓ " } else { "✗ " };
            let color = if file.is_resolved {
                Color::Green
            } else {
                Color::Red
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

/// Rend le panneau de résolution.
fn render_resolution_panel(frame: &mut Frame, state: &ConflictsState, area: Rect) {
    let block = Block::default()
        .title("Résolution du conflit")
        .borders(Borders::ALL)
        .border_style(Style::default());

    let Some(current_file) = state.all_files.get(state.file_selected) else {
        let empty = Paragraph::new("Sélectionnez un fichier")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(empty, area);
        return;
    };

    if current_file.conflicts.is_empty() {
        let empty = Paragraph::new("Aucun conflit dans ce fichier")
            .block(block)
            .style(Style::default().fg(Color::Green));
        frame.render_widget(empty, area);
        return;
    }

    // Construire le contenu avec toutes les sections
    let mut lines: Vec<Line> = Vec::new();

    for (idx, section) in current_file.conflicts.iter().enumerate() {
        let is_selected = idx == state.section_selected;

        // Séparateur entre sections
        if idx > 0 {
            lines.push(Line::from(vec![Span::styled(
                "─".repeat(area.width as usize - 2),
                Style::default().fg(Color::DarkGray),
            )]));
        }

        // Titre de la section
        let section_title = format!("Conflit #{}/{}", idx + 1, current_file.conflicts.len());
        lines.push(Line::from(vec![Span::styled(
            section_title,
            Style::default()
                .fg(if is_selected {
                    Color::Yellow
                } else {
                    Color::Gray
                })
                .add_modifier(Modifier::BOLD),
        )]));

        // Lignes de contexte avant
        if !section.context_before.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "...",
                Style::default().fg(Color::DarkGray),
            )]));
            for line in &section.context_before {
                lines.push(Line::from(vec![Span::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::DarkGray),
                )]));
            }
        }

        // Affichage selon la résolution
        match section.resolution {
            Some(ConflictResolution::Ours) => {
                lines.push(Line::from(vec![Span::styled(
                    "◄─ Ours (sélectionné)",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]));
                for line in &section.ours {
                    lines.push(Line::from(vec![Span::styled(
                        format!("> {}", line),
                        Style::default().fg(Color::Green),
                    )]));
                }
            }
            Some(ConflictResolution::Theirs) => {
                lines.push(Line::from(vec![Span::styled(
                    "─► Theirs (sélectionné)",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )]));
                for line in &section.theirs {
                    lines.push(Line::from(vec![Span::styled(
                        format!("> {}", line),
                        Style::default().fg(Color::Blue),
                    )]));
                }
            }
            Some(ConflictResolution::Both) => {
                lines.push(Line::from(vec![Span::styled(
                    "◄─ Both ─► (sélectionné)",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(vec![Span::styled(
                    "Ours:",
                    Style::default().fg(Color::Green),
                )]));
                for line in &section.ours {
                    lines.push(Line::from(vec![Span::styled(
                        format!("> {}", line),
                        Style::default().fg(Color::Green),
                    )]));
                }
                lines.push(Line::from(vec![Span::styled(
                    "Theirs:",
                    Style::default().fg(Color::Blue),
                )]));
                for line in &section.theirs {
                    lines.push(Line::from(vec![Span::styled(
                        format!("> {}", line),
                        Style::default().fg(Color::Blue),
                    )]));
                }
            }
            None => {
                // Affichage des deux versions (non résolu)
                lines.push(Line::from(vec![Span::styled(
                    "<<<<<<< Ours (HEAD)",
                    Style::default().fg(Color::Green),
                )]));
                for line in &section.ours {
                    lines.push(Line::from(vec![Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::Green),
                    )]));
                }
                lines.push(Line::from(vec![Span::styled(
                    "═══════",
                    Style::default().fg(Color::Yellow),
                )]));
                for line in &section.theirs {
                    lines.push(Line::from(vec![Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::Blue),
                    )]));
                }
                lines.push(Line::from(vec![Span::styled(
                    ">>>>>>> Theirs",
                    Style::default().fg(Color::Blue),
                )]));
            }
        }

        // Lignes de contexte après
        if !section.context_after.is_empty() {
            for line in &section.context_after {
                lines.push(Line::from(vec![Span::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::DarkGray),
                )]));
            }
            lines.push(Line::from(vec![Span::styled(
                "...",
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Rend une vue compacte de la barre de navigation pour la status bar.
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

/// Rend l'overlay d'aide pour la vue conflits.
pub fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(70, 70, area);

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
        Line::from("  ↑/↓ ou j/k  - Naviguer (fichiers / sections / lignes selon le panneau)"),
        Line::from("  Tab         - Panneau suivant (Fichiers → Ours → Theirs → Résultat)"),
        Line::from("  Shift+Tab   - Panneau précédent"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Résolution",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  o           - Garder la version 'ours' (HEAD)"),
        Line::from("  t           - Garder la version 'theirs' (branche mergée)"),
        Line::from("  b           - Garder les deux versions"),
        Line::from("  Enter       - Valider la résolution du fichier courant"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions globales",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  F/B/L       - Changer le mode de résolution (File/Block/Line)"),
        Line::from("  V           - Finaliser le merge (créer le commit)"),
        Line::from("  q ou Esc    - Annuler le merge et revenir au graph"),
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

/// Calcule un rectangle centré de dimensions données (en pourcentage).
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
