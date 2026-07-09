//! Rendu de la vue branches (branches locales/remote, worktrees, stashes).

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::i18n::text;
use crate::state::{
    BranchesFocus, BranchesSection, BranchesViewState, InputAction, SelectedBranch,
};
use crate::ui::common::{centered_rect, StatusBarConfig};
use crate::ui::theme::current_theme;
use crate::utils::time::format_relative_time;

pub struct BranchesRenderContext<'a> {
    pub state: &'a BranchesViewState,
    pub current_branch: Option<&'a str>,
    pub repo_path: &'a str,
    pub flash_message: Option<&'a str>,
}

fn detail_scroll_offset(state: &BranchesViewState) -> u16 {
    state
        .stash_file_diff
        .as_ref()
        .filter(|d| !d.is_empty())
        .map(|_| state.stash_diff_scroll as u16)
        .unwrap_or(0)
}

fn render_detail_panel(
    frame: &mut Frame,
    content: Vec<Line<'static>>,
    scroll_offset: u16,
    area: Rect,
) {
    let theme = current_theme();
    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(text(" Detail ", " Detail "))
                .borders(Borders::ALL),
        )
        .scroll((scroll_offset, 0))
        .style(Style::default().fg(theme.text_normal).bg(theme.background));
    frame.render_widget(paragraph, area);
}

fn render_selection_list<'a>(
    frame: &mut Frame,
    title: &str,
    items: Vec<ListItem<'a>>,
    selected: usize,
    area: Rect,
) {
    let theme = current_theme();
    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_active)),
        )
        .highlight_style(
            Style::default()
                .bg(theme.selection_bg)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().fg(theme.text_normal).bg(theme.background));

    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn append_last_commit_lines(
    lines: &mut Vec<Line<'static>>,
    last_commit_date: Option<std::time::SystemTime>,
    last_commit_message: Option<&str>,
) {
    let theme = current_theme();

    if let Some(date) = last_commit_date {
        let timestamp_secs = date
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        lines.push(Line::from(vec![
            Span::styled(
                text("Modifiee: ", "Updated:  "),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format_relative_time(timestamp_secs),
                Style::default().fg(theme.primary),
            ),
        ]));
    }

    if let Some(msg) = last_commit_message {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            text("Dernier commit:", "Last commit:"),
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(msg.to_string()));
    }
}

fn build_branch_detail_content(state: &BranchesViewState) -> Vec<Line<'static>> {
    let theme = current_theme();

    match state.selected_branch {
        Some(SelectedBranch::Remote(idx)) => {
            if let Some(branch) = state.remote_branches.get(idx) {
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            text("Nom: ", "Name: "),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(branch.name.clone()),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            text("Type: ", "Type: "),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            text("distante", "remote"),
                            Style::default().fg(theme.text_secondary),
                        ),
                    ]),
                ];

                append_last_commit_lines(
                    &mut lines,
                    branch.last_commit_date,
                    branch.last_commit_message.as_deref(),
                );
                lines
            } else {
                vec![Line::from(text(
                    "Aucune branche distante selectionnee",
                    "No remote branch selected",
                ))]
            }
        }
        Some(SelectedBranch::Local(idx)) => {
            if let Some(branch) = state.local_branches.get(idx) {
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            text("Nom: ", "Name: "),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(branch.name.clone()),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            text("HEAD: ", "HEAD: "),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            if branch.is_head {
                                text("oui", "yes")
                            } else {
                                text("non", "no")
                            },
                            if branch.is_head {
                                Style::default().fg(theme.success)
                            } else {
                                Style::default()
                            },
                        ),
                    ]),
                ];

                if let (Some(ahead), Some(behind)) = (branch.ahead, branch.behind) {
                    lines.push(Line::from(vec![
                        Span::styled(
                            text("Ahead/Behind: ", "Ahead/Behind: "),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!("{} / {}", ahead, behind)),
                    ]));
                }

                append_last_commit_lines(
                    &mut lines,
                    branch.last_commit_date,
                    branch.last_commit_message.as_deref(),
                );
                lines
            } else {
                vec![Line::from(text(
                    "Aucune branche locale selectionnee",
                    "No local branch selected",
                ))]
            }
        }
        None => vec![Line::from(text(
            "Aucune branche selectionnee",
            "No branch selected",
        ))],
    }
}

fn build_worktree_detail_content(state: &BranchesViewState) -> Vec<Line<'static>> {
    let theme = current_theme();

    if let Some(worktree) = state.worktrees.get(state.worktree_selected()) {
        vec![
            Line::from(vec![
                Span::styled(
                    text("Nom: ", "Name: "),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(worktree.name.clone()),
            ]),
            Line::from(vec![
                Span::styled(
                    text("Chemin: ", "Path: "),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(worktree.path.clone()),
            ]),
            Line::from(vec![
                Span::styled(
                    text("Principal: ", "Main: "),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if worktree.is_main {
                        text("oui", "yes")
                    } else {
                        text("non", "no")
                    },
                    if worktree.is_main {
                        Style::default().fg(theme.success)
                    } else {
                        Style::default()
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    text("Actif: ", "Active: "),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if worktree.is_current {
                        text("oui", "yes")
                    } else {
                        text("non", "no")
                    },
                    if worktree.is_current {
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    text("Branche: ", "Branch: "),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(worktree.branch.as_deref().unwrap_or("N/A").to_string()),
            ]),
        ]
    } else {
        vec![Line::from(text(
            "Aucun worktree selectionne",
            "No worktree selected",
        ))]
    }
}

fn build_stash_detail_content(state: &BranchesViewState) -> Vec<Line<'static>> {
    let theme = current_theme();

    let Some(stash) = state.stashes.get(state.stash_selected()) else {
        return vec![Line::from(text(
            "Aucun stash selectionne",
            "No stash selected",
        ))];
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                text("Message: ", "Message: "),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(stash.message.clone()),
        ]),
        Line::from(vec![
            Span::styled(
                text("Index: ", "Index: "),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("stash@{{{}}}", stash.index)),
        ]),
    ];

    if let Some(branch) = stash.branch.as_deref() {
        lines.push(Line::from(vec![
            Span::styled(
                text("Branche: ", "Branch: "),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(branch.to_string()),
        ]));
    }

    if !stash.files.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            text("Fichiers modifies:", "Modified files:"),
            Style::default().add_modifier(Modifier::BOLD),
        )]));

        for (i, file) in stash.files.iter().enumerate() {
            let status_color = match file.status_char() {
                'A' => theme.success,
                'M' => theme.warning,
                'D' => theme.error,
                'R' => theme.primary,
                _ => theme.text_secondary,
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
                Span::raw(file.path.clone()),
            ]));
        }

        if let Some(diff_lines) = state.stash_file_diff.as_ref().filter(|d| !d.is_empty()) {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                text("Diff:", "Diff:"),
                Style::default().add_modifier(Modifier::BOLD),
            )]));

            lines.extend(diff_lines.iter().map(|line| {
                let styled_line = if line.starts_with('+') {
                    Span::styled(line.clone(), Style::default().fg(theme.success))
                } else if line.starts_with('-') {
                    Span::styled(line.clone(), Style::default().fg(theme.error))
                } else if line.starts_with('@') {
                    Span::styled(line.clone(), Style::default().fg(theme.primary))
                } else {
                    Span::raw(line.clone())
                };
                Line::from(styled_line)
            }));
        }
    }

    lines
}

/// Rend la vue complète branches/worktrees/stashes.
pub fn render(frame: &mut Frame, ctx: BranchesRenderContext<'_>) {
    let BranchesRenderContext {
        state,
        current_branch,
        repo_path,
        flash_message,
    } = ctx;

    let layout = super::branches_layout::build_branches_layout(frame.area());

    // Status bar.
    crate::ui::common::render_status_bar(
        frame,
        StatusBarConfig {
            view_title: text("branches", "branches"),
            branch: current_branch,
            repo_path,
            flash_message,
            bg_color: None,
        },
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

/// Rend les onglets de la vue branches.
fn render_tabs(frame: &mut Frame, active: &BranchesSection, area: Rect) {
    let theme = current_theme();
    let tabs = vec![
        (text("Branches", "Branches"), BranchesSection::Branches),
        (text("Worktrees", "Worktrees"), BranchesSection::Worktrees),
        (text("Stashes", "Stashes"), BranchesSection::Stashes),
    ];

    let mut spans = Vec::new();
    for (label, section) in &tabs {
        let style = if section == active {
            Style::default()
                .fg(theme.border_active)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.text_secondary)
        };
        spans.push(Span::styled(format!(" {} ", label), style));
        spans.push(Span::styled("  ", Style::default().fg(theme.text_normal)));
    }

    let line = Line::from(spans);
    let paragraph =
        Paragraph::new(line).style(Style::default().fg(theme.text_normal).bg(theme.background));
    frame.render_widget(paragraph, area);
}

/// Rend la liste des branches.
fn render_branches_list(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let theme = current_theme();
    let mut items: Vec<ListItem> = Vec::new();

    // Section locale.
    items.push(ListItem::new(Line::from(Span::styled(
        text("Local", "Local"),
        Style::default()
            .fg(theme.commit_hash)
            .add_modifier(Modifier::BOLD),
    ))));

    for branch in state.local_branches.iter() {
        let prefix = if branch.is_head { "● " } else { "  " };
        let style = if branch.is_head {
            Style::default()
                .fg(theme.success)
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
                Style::default().fg(theme.text_secondary),
            ));
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    // Section remote (si activée).
    if state.show_remote {
        items.push(ListItem::new(Line::from("")));
        items.push(ListItem::new(Line::from(Span::styled(
            text("Distant", "Remote"),
            Style::default()
                .fg(theme.commit_hash)
                .add_modifier(Modifier::BOLD),
        ))));

        for branch in &state.remote_branches {
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(&branch.name, Style::default().fg(theme.text_secondary)),
            ])));
        }
    }

    let local_count = state.local_branches.len();

    // Calculer l'index visuel basé sur la sélection explicite
    let visual_index = match state.selected_branch {
        Some(SelectedBranch::Local(idx)) => {
            // Header "Local" + index dans les branches locales
            idx + 1
        }
        Some(SelectedBranch::Remote(idx)) => {
            // Header "Local" + branches locales + ligne vide + header "Remote" + index
            local_count + idx + 3
        }
        None => 0,
    };
    render_selection_list(
        frame,
        text(" Branches ", " Branches "),
        items,
        visual_index,
        area,
    );
}

/// Rend le détail d'une branche.
fn render_branch_detail(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    render_detail_panel(
        frame,
        build_branch_detail_content(state),
        detail_scroll_offset(state),
        area,
    );
}

/// Rend la liste des worktrees.
fn render_worktrees_list(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let theme = current_theme();
    let items: Vec<ListItem> = state
        .worktrees
        .iter()
        .map(|worktree| {
            let prefix = if worktree.is_current { "● " } else { "  " };
            let style = if worktree.is_current {
                Style::default()
                    .fg(theme.success)
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

    render_selection_list(
        frame,
        text(" Worktrees ", " Worktrees "),
        items,
        state.worktree_selected(),
        area,
    );
}

/// Rend le détail d'un worktree.
fn render_worktree_detail(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    render_detail_panel(
        frame,
        build_worktree_detail_content(state),
        detail_scroll_offset(state),
        area,
    );
}

/// Rend la liste des stashes.
fn render_stashes_list(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let theme = current_theme();
    let items: Vec<ListItem> = state
        .stashes
        .iter()
        .map(|stash| {
            let line = Line::from(vec![
                Span::styled(
                    format!("stash@{{{}}}: ", stash.index),
                    Style::default().fg(theme.primary),
                ),
                Span::raw(&stash.message),
            ]);
            ListItem::new(line)
        })
        .collect();

    render_selection_list(
        frame,
        text(" Stashes ", " Stashes "),
        items,
        state.stash_selected(),
        area,
    );
}

/// Rend le détail d'un stash.
fn render_stash_detail(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    render_detail_panel(
        frame,
        build_stash_detail_content(state),
        detail_scroll_offset(state),
        area,
    );
}

/// Rend la barre d'aide de la vue branches.
fn render_branches_help(
    frame: &mut Frame,
    section: &BranchesSection,
    focus: &BranchesFocus,
    area: Rect,
) {
    let theme = current_theme();
    let help_text = if *focus == BranchesFocus::Input {
        text(
            "Entree:confirmer  Echap:annuler  ←→:curseur",
            "Enter:confirm  Esc:cancel  ←→:cursor",
        )
    } else {
        match section {
            BranchesSection::Branches => {
                text(
                    "Tab/Shift+Tab:section  Entree:checkout  n:nouvelle  d:supprimer  r:renommer  m:fusion  e:rebase  R:distantes  P:push  Ctrl+P:force push  1:graphe  2:staging",
                    "Tab/Shift+Tab:section  Enter:checkout  n:new  d:delete  r:rename  m:merge  e:rebase  R:remote  P:push  Ctrl+P:force push  1:graph  2:staging",
                )
            }
            BranchesSection::Worktrees => {
                text(
                    "↑↓:selectionner  Entree:ouvrir  Tab/Shift+Tab:section  n:nouveau  d:supprimer  1:graphe",
                    "↑↓:select  Enter:open  Tab/Shift+Tab:section  n:new  d:delete  1:graph",
                )
            }
            BranchesSection::Stashes => {
                text(
                    "Tab/Shift+Tab:section  h/l:fichiers  J/K:defiler diff  a:appliquer  p:pop  d:supprimer  s:sauver  1:graphe  2:staging",
                    "Tab/Shift+Tab:section  h/l:files  J/K:scroll diff  a:apply  p:pop  d:drop  s:save  1:graph  2:staging",
                )
            }
        }
    };

    let line = Line::from(vec![Span::styled(
        format!(" {} ", help_text),
        Style::default().fg(theme.text_secondary),
    )]);

    frame.render_widget(Paragraph::new(line), area);
}

/// Rend l'overlay d'input.
fn render_input_overlay(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let theme = current_theme();
    let popup = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup);

    let title = match state.input_action {
        Some(InputAction::CreateBranch) => text(" Nouvelle branche ", " New branch "),
        Some(InputAction::RenameBranch) => text(" Renommer la branche ", " Rename branch "),
        Some(InputAction::CreateWorktree) => text(
            " Nouveau worktree: nom chemin [branche] ",
            " New worktree: name path [branch] ",
        ),
        Some(InputAction::SaveStash) => text(" Message du stash ", " Stash message "),
        None => text(" Saisie ", " Input "),
    };

    let input = crate::ui::text_edit::text_with_selection(
        &state.input_text,
        state.input_cursor,
        state.input_selection_anchor,
        Style::default().fg(theme.text_normal).bg(theme.background),
        Style::default()
            .fg(theme.selection_fg)
            .bg(theme.selection_bg),
    );
    let paragraph = Paragraph::new(input)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.warning)),
        )
        .style(Style::default().fg(theme.text_normal).bg(theme.background));

    frame.render_widget(paragraph, popup);

    // Curseur.
    let before_cursor: String = state.input_text.chars().take(state.input_cursor).collect();
    let cursor_width = Line::from(before_cursor).width() as u16;
    frame.set_cursor_position((popup.x + cursor_width + 1, popup.y + 1));
}
