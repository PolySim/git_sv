use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{BranchesFocus, BranchesSection, BranchesViewState, InputAction};
use crate::ui::common::centered_rect;

/// Rend la vue complète branches/worktrees/stashes.
pub fn render(
    frame: &mut Frame,
    state: &BranchesViewState,
    current_branch: &Option<String>,
    repo_path: &str,
    flash_message: Option<&str>,
) {
    let layout = super::branches_layout::build_branches_layout(frame.area());

    // Status bar.
    render_branches_status_bar(
        frame,
        current_branch,
        repo_path,
        flash_message,
        layout.status_bar,
    );

    // Onglets.
    render_tabs(frame, &state.section, layout.tabs);

    // Contenu selon la section active.
    match state.section {
        BranchesSection::Branches => {
            render_branches_list(frame, state, layout.list_panel);
            render_branch_detail(frame, state, layout.detail_panel);
        }
        BranchesSection::Worktrees => {
            render_worktrees_list(frame, state, layout.list_panel);
            render_worktree_detail(frame, state, layout.detail_panel);
        }
        BranchesSection::Stashes => {
            render_stashes_list(frame, state, layout.list_panel);
            render_stash_detail(frame, state, layout.detail_panel);
        }
    }

    // Help bar contextuelle.
    render_branches_help(frame, &state.section, &state.focus, layout.help_bar);

    // Overlay d'input si actif.
    if state.focus == BranchesFocus::Input {
        render_input_overlay(frame, state, frame.area());
    }
}

/// Rend la status bar de la vue branches.
fn render_branches_status_bar(
    frame: &mut Frame,
    current_branch: &Option<String>,
    repo_path: &str,
    flash_message: Option<&str>,
    area: Rect,
) {
    let branch_name = current_branch.as_deref().unwrap_or("???");

    let content = if let Some(msg) = flash_message {
        format!(
            " git_sv · branches · {} · {} · {} ",
            repo_path, branch_name, msg
        )
    } else {
        format!(" git_sv · branches · {} · {} ", repo_path, branch_name)
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

/// Rend les onglets de la vue branches.
fn render_tabs(frame: &mut Frame, active: &BranchesSection, area: Rect) {
    let tabs = vec![
        ("Branches", BranchesSection::Branches),
        ("Worktrees", BranchesSection::Worktrees),
        ("Stashes", BranchesSection::Stashes),
    ];

    let mut spans = Vec::new();
    for (label, section) in &tabs {
        let style = if section == active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {} ", label), style));
        spans.push(Span::raw("  "));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

/// Rend la liste des branches.
fn render_branches_list(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    // Section locale.
    items.push(ListItem::new(Line::from(Span::styled(
        "Local",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))));

    for (_i, branch) in state.local_branches.iter().enumerate() {
        let prefix = if branch.is_head { "● " } else { "  " };
        let style = if branch.is_head {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let mut spans = vec![
            Span::styled(prefix, style),
            Span::styled(&branch.name, style),
        ];

        // Ahead/Behind si disponible.
        if let (Some(ahead), Some(behind)) = (branch.ahead, branch.behind) {
            spans.push(Span::styled(
                format!("  {}↑ {}↓", ahead, behind),
                Style::default().fg(Color::DarkGray),
            ));
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    // Section remote (si activée).
    if state.show_remote {
        items.push(ListItem::new(Line::from("")));
        items.push(ListItem::new(Line::from(Span::styled(
            "Remote",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))));

        for branch in &state.remote_branches {
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(&branch.name, Style::default().fg(Color::DarkGray)),
            ])));
        }
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Branches ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    let local_count = state.local_branches.len();
    let remote_count = state.remote_branches.len();
    let show_remote = state.show_remote;

    let visual_index = if show_remote && remote_count > 0 {
        if local_count > 0 {
            if state.branch_selected < local_count {
                state.branch_selected + 1
            } else {
                state.branch_selected + 3
            }
        } else {
            state.branch_selected + 1
        }
    } else {
        state.branch_selected + 1
    };
    list_state.select(Some(visual_index));
    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Rend le détail d'une branche.
fn render_branch_detail(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let local_count = state.local_branches.len();
    let is_remote = state.show_remote
        && !state.remote_branches.is_empty()
        && state.branch_selected >= local_count;

    let content = if is_remote {
        if let Some(branch) = state
            .remote_branches
            .get(state.branch_selected - local_count)
        {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Nom: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&branch.name),
                ]),
                Line::from(vec![
                    Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled("distante", Style::default().fg(Color::DarkGray)),
                ]),
            ];

            if let Some(ref msg) = branch.last_commit_message {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Dernier commit:",
                    Style::default().add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(msg.as_str()));
            }

            lines
        } else {
            vec![Line::from("Aucune branche distante sélectionnée")]
        }
    } else if let Some(branch) = state.local_branches.get(state.branch_selected) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Nom: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&branch.name),
            ]),
            Line::from(vec![
                Span::styled("HEAD: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if branch.is_head { "oui" } else { "non" },
                    if branch.is_head {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    },
                ),
            ]),
        ];

        if let (Some(ahead), Some(behind)) = (branch.ahead, branch.behind) {
            lines.push(Line::from(vec![
                Span::styled(
                    "Ahead/Behind: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{} / {}", ahead, behind)),
            ]));
        }

        if let Some(ref msg) = branch.last_commit_message {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Dernier commit:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(msg.as_str()));
        }

        lines
    } else {
        vec![Line::from("Aucune branche sélectionnée")]
    };

    let paragraph =
        Paragraph::new(content).block(Block::default().title(" Détail ").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

/// Rend la liste des worktrees.
fn render_worktrees_list(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let items: Vec<ListItem> = state
        .worktrees
        .iter()
        .enumerate()
        .map(|(_i, worktree)| {
            let prefix = if worktree.is_main { "● " } else { "  " };
            let style = if worktree.is_main {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&worktree.name, style),
            ])
        })
        .map(ListItem::new)
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Worktrees ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(state.worktree_selected));
    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Rend le détail d'un worktree.
fn render_worktree_detail(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let content = if let Some(worktree) = state.worktrees.get(state.worktree_selected) {
        vec![
            Line::from(vec![
                Span::styled("Nom: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&worktree.name),
            ]),
            Line::from(vec![
                Span::styled("Chemin: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&worktree.path),
            ]),
            Line::from(vec![
                Span::styled("Principal: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if worktree.is_main { "oui" } else { "non" },
                    if worktree.is_main {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Branche: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(worktree.branch.as_deref().unwrap_or("N/A")),
            ]),
        ]
    } else {
        vec![Line::from("Aucun worktree sélectionné")]
    };

    let paragraph =
        Paragraph::new(content).block(Block::default().title(" Détail ").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

/// Rend la liste des stashes.
fn render_stashes_list(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let items: Vec<ListItem> = state
        .stashes
        .iter()
        .enumerate()
        .map(|(_i, stash)| {
            let line = Line::from(vec![
                Span::styled(
                    format!("stash@{{{}}}: ", stash.index),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(&stash.message),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Stashes ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(state.stash_selected));
    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Rend le détail d'un stash.
fn render_stash_detail(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let content = if let Some(stash) = state.stashes.get(state.stash_selected) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Message: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&stash.message),
            ]),
            Line::from(vec![
                Span::styled("Index: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("stash@{{{}}}", stash.index)),
            ]),
        ];

        if let Some(ref branch) = stash.branch {
            lines.push(Line::from(vec![
                Span::styled("Branche: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(branch),
            ]));
        }

        if !stash.files.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Fichiers modifiés:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));

            for (i, file) in stash.files.iter().enumerate() {
                let status_color = match file.status_char() {
                    'A' => Color::Green,
                    'M' => Color::Yellow,
                    'D' => Color::Red,
                    'R' => Color::Cyan,
                    _ => Color::White,
                };
                let is_selected = i == state.stash_file_selected;
                let prefix = if is_selected { "→ " } else { "  " };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}{} ", prefix, file.status_char()),
                        Style::default()
                            .fg(status_color)
                            .add_modifier(if is_selected {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                    Span::raw(&file.path),
                ]));
            }

            // Afficher le diff du fichier sélectionné
            if let Some(ref diff_lines) = state.stash_file_diff {
                if !diff_lines.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![Span::styled(
                        "Diff:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )]));

                    // Limiter le nombre de lignes affichées
                    let max_lines = 30;
                    for line in diff_lines.iter().take(max_lines) {
                        let styled_line = if line.starts_with('+') {
                            Span::styled(line, Style::default().fg(Color::Green))
                        } else if line.starts_with('-') {
                            Span::styled(line, Style::default().fg(Color::Red))
                        } else {
                            Span::raw(line)
                        };
                        lines.push(Line::from(styled_line));
                    }

                    if diff_lines.len() > max_lines {
                        lines.push(Line::from(Span::styled(
                            format!("... ({} lignes masquées)", diff_lines.len() - max_lines),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
            }
        }

        lines
    } else {
        vec![Line::from("Aucun stash sélectionné")]
    };

    let paragraph =
        Paragraph::new(content).block(Block::default().title(" Détail ").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

/// Rend la barre d'aide de la vue branches.
fn render_branches_help(
    frame: &mut Frame,
    section: &BranchesSection,
    focus: &BranchesFocus,
    area: Rect,
) {
    let help_text = if *focus == BranchesFocus::Input {
        "Enter:confirmer  Esc:annuler  ←→:curseur"
    } else {
        match section {
            BranchesSection::Branches => {
                "Tab:section  Enter:checkout  n:new  d:delete  r:rename  m:merge  R:remote  P:push  1:graph  2:staging"
            }
            BranchesSection::Worktrees => {
                "Tab:section  n:new  d:delete  1:graph  2:staging"
            }
            BranchesSection::Stashes => {
                "Tab:section  h/l:fichiers  a:apply  p:pop  d:drop  s:save  1:graph  2:staging"
            }
        }
    };

    let line = Line::from(vec![Span::styled(
        format!(" {} ", help_text),
        Style::default().fg(Color::DarkGray),
    )]);

    frame.render_widget(Paragraph::new(line), area);
}

/// Rend l'overlay d'input.
fn render_input_overlay(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let popup = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup);

    let title = match state.input_action {
        Some(InputAction::CreateBranch) => " Nouvelle branche ",
        Some(InputAction::RenameBranch) => " Renommer la branche ",
        Some(InputAction::CreateWorktree) => " Nouveau worktree (nom chemin [branche]) ",
        Some(InputAction::SaveStash) => " Message du stash ",
        None => " Input ",
    };

    let paragraph = Paragraph::new(state.input_text.as_str()).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(paragraph, popup);

    // Curseur.
    frame.set_cursor_position((popup.x + state.input_cursor as u16 + 1, popup.y + 1));
}
