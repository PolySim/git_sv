use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::state::action::{BranchAction, NavigationAction, StagingAction};
use crate::state::{AppAction, AppState, FocusPanel, ProjectTreeAction, ViewMode};

pub(crate) fn map_mouse(mouse: MouseEvent, state: &AppState) -> Option<AppAction> {
    use crate::ui::hit_test::hit_test;

    let x = mouse.column;
    let y = mouse.row;

    // Détecter la zone cliquée
    let hit_result = hit_test(state, x, y)?;
    if hit_result.zone == crate::ui::hit_test::ClickableZone::Modal {
        return match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => handle_modal_click(state, &hit_result),
            _ => None,
        };
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Gérer le clic selon la zone
            handle_mouse_click(state, hit_result)
        }
        MouseEventKind::ScrollUp => {
            // Scroll up dans la zone concernée
            handle_mouse_scroll(state, hit_result, true)
        }
        MouseEventKind::ScrollDown => {
            // Scroll down dans la zone concernée
            handle_mouse_scroll(state, hit_result, false)
        }
        _ => None,
    }
}

/// Gère un clic souris en fonction de la zone.
fn handle_mouse_click(
    state: &AppState,
    hit: crate::ui::hit_test::HitTestResult,
) -> Option<AppAction> {
    use crate::ui::hit_test::{calculate_commit_index, calculate_nav_tab, ClickableZone};

    if state.view_mode == ViewMode::Staging {
        return handle_staging_mouse_click(state, &hit);
    }

    if state.view_mode == ViewMode::Branches {
        return handle_branches_mouse_click(state, &hit);
    }

    if state.view_mode == ViewMode::ProjectTree {
        return handle_project_tree_mouse_click(state, &hit);
    }

    match hit.zone {
        ClickableZone::Modal => handle_modal_click(state, &hit),
        ClickableZone::NavBar => {
            // Clic sur un tab de navigation
            let conflicts = state
                .conflicts_state
                .as_ref()
                .map(|value| {
                    value
                        .all_files
                        .iter()
                        .filter(|file| !file.is_resolved)
                        .count()
                })
                .unwrap_or(0);
            calculate_nav_tab(hit.relative_x, conflicts).map(AppAction::SwitchView)
        }
        ClickableZone::Graph => {
            // Clic dans le graphe: sélectionner un commit et mettre le focus sur Graph
            let selected_index = state.graph_view.selected_index();

            // Calculer l'offset de scroll visible (approximation)
            let visible_height = hit.rect.height.saturating_sub(2) as usize;
            let visible_commits = (visible_height / 2).max(1);
            let scroll_offset = selected_index.saturating_sub(visible_commits / 2);

            if let Some(commit_index) = hit.relative_y.checked_sub(1).and_then(|row| {
                (row < hit.rect.height.saturating_sub(2))
                    .then(|| {
                        calculate_commit_index(state.graph_view.rows.items(), scroll_offset, row)
                            .filter(|&index| index < scroll_offset + visible_commits)
                    })
                    .flatten()
            }) {
                // Sélectionner le commit cliqué directement par son index
                Some(AppAction::Navigation(
                    crate::state::action::NavigationAction::SelectCommit(commit_index),
                ))
            } else {
                // Clic en dehors des commits visibles, juste mettre le focus sur Graph
                Some(AppAction::Navigation(
                    crate::state::action::NavigationAction::FocusGraph,
                ))
            }
        }
        ClickableZone::BottomLeft => {
            // Clic dans le panneau fichiers: sélectionner un fichier
            let file_count = state.graph_view.commit_files.len();

            if let Some(file_index) = visible_list_index(&hit, state.graph_view.file_selected_index)
                .filter(|&index| index < file_count)
            {
                // Sélectionner le fichier cliqué directement par son index
                Some(AppAction::Navigation(
                    crate::state::action::NavigationAction::SelectFile(file_index),
                ))
            } else {
                // Clic en dehors des fichiers, juste mettre le focus sur BottomLeft
                if state.focus != FocusPanel::BottomLeft {
                    Some(AppAction::Navigation(
                        crate::state::action::NavigationAction::FocusBottomLeft,
                    ))
                } else {
                    None
                }
            }
        }
        ClickableZone::BottomRight => {
            // Clic dans le panneau diff: mettre le focus
            if state.focus != FocusPanel::BottomRight {
                Some(AppAction::Navigation(
                    crate::state::action::NavigationAction::FocusBottomRight,
                ))
            } else {
                None
            }
        }
        ClickableZone::SearchBar => {
            // Clic dans la barre de recherche - pourrait positionner le curseur
            None
        }
        ClickableZone::HelpBar => {
            // Clic dans la barre d'aide - pourrait afficher l'aide
            Some(AppAction::ToggleHelp)
        }
        ClickableZone::StatusBar => {
            // Clic dans la status bar
            None
        }
        ClickableZone::Outside => None,
    }
}

fn handle_project_tree_mouse_click(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
) -> Option<AppAction> {
    use crate::ui::project_tree_layout::build_project_tree_layout;

    let layout =
        build_project_tree_layout(state.screen_area, state.project_tree_state.search.is_active);
    if hit.rect == layout.nav_bar {
        return calculate_global_nav_action(state, hit.relative_x);
    }

    let item_y = hit.relative_y.saturating_sub(1) as usize;
    if hit.rect == layout.tree_panel {
        if state.project_tree_state.search.is_active {
            let results = &state.project_tree_state.search.results;
            let index = results.scroll_offset() + item_y;
            return (index < results.len()).then_some(AppAction::ProjectTree(
                ProjectTreeAction::SelectSearchResult(index),
            ));
        }
        let entries = &state.project_tree_state.entries;
        let index = entries.scroll_offset() + item_y;
        return (index < entries.len()).then_some(AppAction::ProjectTree(
            ProjectTreeAction::ActivateTreeEntry(index),
        ));
    }

    if hit.rect == layout.history_panel {
        let index = state.project_tree_state.history.scroll_offset() + item_y;
        return Some(AppAction::ProjectTree(
            if index < state.project_tree_state.history.len() {
                ProjectTreeAction::SelectHistoryEntry(index)
            } else {
                ProjectTreeAction::FocusHistory
            },
        ));
    }

    if hit.rect == layout.changed_files_panel {
        return Some(AppAction::ProjectTree(
            match state
                .project_tree_state
                .changed_file_index_at_visual_row(item_y)
            {
                Some(index) => ProjectTreeAction::SelectChangedFile(index),
                None => ProjectTreeAction::FocusChangedFiles,
            },
        ));
    }

    if hit.rect == layout.diff_panel {
        return Some(AppAction::ProjectTree(ProjectTreeAction::FocusDiff));
    }

    if hit.rect == layout.help_bar {
        return Some(AppAction::ToggleHelp);
    }

    None
}

fn handle_modal_click(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
) -> Option<AppAction> {
    if state.ui.pending_confirmation.is_some() {
        if crate::ui::confirm_dialog::confirm_button_contains(
            state.screen_area,
            hit.relative_x,
            hit.relative_y,
        ) {
            return Some(AppAction::ConfirmAction);
        }

        return Some(AppAction::CancelAction);
    }

    None
}

fn handle_staging_mouse_click(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
) -> Option<AppAction> {
    use crate::state::StagingFocus;
    use crate::ui::staging_layout::build_staging_layout;

    let layout = build_staging_layout(state.screen_area);

    if hit.rect == layout.nav_bar {
        return calculate_global_nav_action(state, hit.relative_x);
    }

    if hit.rect == layout.unstaged_panel {
        let item_y = visible_list_index(hit, state.staging_state.unstaged_selected())?;
        if item_y < state.staging_state.unstaged_files().len() {
            return Some(AppAction::Staging(StagingAction::SelectUnstaged(item_y)));
        }
        return match state.staging_state.focus {
            StagingFocus::Unstaged => None,
            _ => Some(AppAction::Staging(StagingAction::FocusUnstaged)),
        };
    }

    if hit.rect == layout.staged_panel {
        let item_y = visible_list_index(hit, state.staging_state.staged_selected())?;
        if item_y < state.staging_state.staged_files().len() {
            return Some(AppAction::Staging(StagingAction::SelectStaged(item_y)));
        }
        return match state.staging_state.focus {
            StagingFocus::Staged => None,
            _ => Some(AppAction::Staging(StagingAction::FocusStaged)),
        };
    }

    if hit.rect == layout.diff_panel {
        return Some(AppAction::Staging(StagingAction::FocusDiff));
    }

    if hit.rect == layout.commit_message && state.staging_state.focus != StagingFocus::CommitMessage
    {
        return Some(AppAction::Staging(StagingAction::StartCommitMessage));
    }

    None
}

fn handle_branches_mouse_click(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
) -> Option<AppAction> {
    use crate::state::BranchesSection;
    use crate::ui::branches_layout::build_branches_layout;

    let layout = build_branches_layout(state.screen_area);

    if hit.rect == layout.nav_bar {
        return calculate_global_nav_action(state, hit.relative_x);
    }

    if hit.rect == layout.tabs {
        let third = (layout.tabs.width / 3).max(1);
        let tab = hit.relative_x / third;

        return match (state.branches_view_state.section, tab) {
            (BranchesSection::Branches, 0) => None,
            (BranchesSection::Branches, 1) => Some(AppAction::Branch(BranchAction::NextSection)),
            (BranchesSection::Branches, 2) => Some(AppAction::Branch(BranchAction::PrevSection)),
            (BranchesSection::Worktrees, 0) => Some(AppAction::Branch(BranchAction::PrevSection)),
            (BranchesSection::Worktrees, 1) => None,
            (BranchesSection::Worktrees, 2) => Some(AppAction::Branch(BranchAction::NextSection)),
            (BranchesSection::Stashes, 0) => Some(AppAction::Branch(BranchAction::NextSection)),
            (BranchesSection::Stashes, 1) => Some(AppAction::Branch(BranchAction::PrevSection)),
            (BranchesSection::Stashes, 2) => None,
            _ => None,
        };
    }

    if hit.rect == layout.list_panel {
        let selected = match state.branches_view_state.section {
            BranchesSection::Branches => match state.branches_view_state.selected_branch {
                Some(crate::state::SelectedBranch::Local(index)) => index + 1,
                Some(crate::state::SelectedBranch::Remote(index)) => {
                    state.branches_view_state.local_branches.len() + index + 3
                }
                None => 0,
            },
            BranchesSection::Worktrees => state.branches_view_state.worktree_selected(),
            BranchesSection::Stashes => state.branches_view_state.stash_selected(),
        };
        let item_y = visible_list_index(hit, selected)?;

        return match state.branches_view_state.section {
            BranchesSection::Branches => {
                if item_y == 0 {
                    Some(AppAction::Branch(BranchAction::FocusList))
                } else if item_y <= state.branches_view_state.local_branches.len() {
                    Some(AppAction::Branch(BranchAction::SelectLocalBranch(
                        item_y - 1,
                    )))
                } else if state.branches_view_state.show_remote {
                    let remote_start = state.branches_view_state.local_branches.len() + 3;
                    if item_y >= remote_start {
                        let remote_index = item_y - remote_start;
                        if remote_index < state.branches_view_state.remote_branches.len() {
                            Some(AppAction::Branch(BranchAction::SelectRemoteBranch(
                                remote_index,
                            )))
                        } else {
                            Some(AppAction::Branch(BranchAction::FocusList))
                        }
                    } else {
                        Some(AppAction::Branch(BranchAction::FocusList))
                    }
                } else {
                    Some(AppAction::Branch(BranchAction::FocusList))
                }
            }
            BranchesSection::Worktrees => {
                if item_y < state.branches_view_state.worktrees.len() {
                    Some(AppAction::Branch(BranchAction::SelectWorktree(item_y)))
                } else {
                    Some(AppAction::Branch(BranchAction::FocusList))
                }
            }
            BranchesSection::Stashes => {
                if item_y < state.branches_view_state.stashes.len() {
                    Some(AppAction::Branch(BranchAction::SelectStash(item_y)))
                } else {
                    Some(AppAction::Branch(BranchAction::FocusList))
                }
            }
        };
    }

    if hit.rect == layout.detail_panel {
        return Some(AppAction::Branch(BranchAction::FocusDetail));
    }

    None
}

/// Retrouve une ligne dans les listes rendues depuis un ListState initialement vide.
fn visible_list_index(hit: &crate::ui::hit_test::HitTestResult, selected: usize) -> Option<usize> {
    let row = hit.relative_y.checked_sub(1)? as usize;
    let height = hit.rect.height.saturating_sub(2) as usize;
    if row >= height {
        return None;
    }
    Some(selected.saturating_sub(height.saturating_sub(1)) + row)
}

fn calculate_global_nav_action(state: &AppState, relative_x: u16) -> Option<AppAction> {
    let conflicts = state
        .conflicts_state
        .as_ref()
        .map(|value| {
            value
                .all_files
                .iter()
                .filter(|file| !file.is_resolved)
                .count()
        })
        .unwrap_or(0);
    crate::ui::hit_test::calculate_nav_tab(relative_x, conflicts).map(AppAction::SwitchView)
}

/// Gère le scroll souris en fonction de la zone.
fn handle_mouse_scroll(
    state: &AppState,
    hit: crate::ui::hit_test::HitTestResult,
    is_up: bool,
) -> Option<AppAction> {
    use crate::ui::hit_test::ClickableZone;

    if state.view_mode == ViewMode::Staging {
        return handle_staging_mouse_scroll(state, &hit, is_up);
    }

    if state.view_mode == ViewMode::ProjectTree {
        return handle_project_tree_mouse_scroll(state, &hit, is_up);
    }

    let action = match hit.zone {
        ClickableZone::Graph => {
            if is_up {
                AppAction::Navigation(NavigationAction::MoveUp)
            } else {
                AppAction::Navigation(NavigationAction::MoveDown)
            }
        }
        ClickableZone::BottomLeft => {
            if is_up {
                AppAction::Navigation(NavigationAction::FileUp)
            } else {
                AppAction::Navigation(NavigationAction::FileDown)
            }
        }
        ClickableZone::BottomRight => {
            if is_up {
                AppAction::Navigation(NavigationAction::ScrollDiffUp)
            } else {
                AppAction::Navigation(NavigationAction::ScrollDiffDown)
            }
        }
        _ => {
            // Par défaut, utiliser le focus actuel
            return handle_scroll_with_current_focus(state, is_up);
        }
    };

    Some(action)
}

fn handle_project_tree_mouse_scroll(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
    is_up: bool,
) -> Option<AppAction> {
    use crate::ui::project_tree_layout::build_project_tree_layout;

    let layout =
        build_project_tree_layout(state.screen_area, state.project_tree_state.search.is_active);
    if hit.rect == layout.tree_panel {
        let selection = if state.project_tree_state.search.is_active {
            &state.project_tree_state.search.results
        } else {
            &state.project_tree_state.entries
        };
        let current = selection.selected_index();
        let next = if is_up {
            current.saturating_sub(1)
        } else {
            (current + 1).min(selection.len().saturating_sub(1))
        };
        let action = if state.project_tree_state.search.is_active {
            ProjectTreeAction::SelectSearchResult(next)
        } else {
            ProjectTreeAction::SelectTreeEntry(next)
        };
        return Some(AppAction::ProjectTree(action));
    }
    if hit.rect == layout.history_panel {
        let current = state.project_tree_state.history.selected_index();
        let next = if is_up {
            current.saturating_sub(1)
        } else {
            (current + 1).min(state.project_tree_state.history.len().saturating_sub(1))
        };
        return Some(AppAction::ProjectTree(
            ProjectTreeAction::SelectHistoryEntry(next),
        ));
    }
    if hit.rect == layout.changed_files_panel {
        let files = &state.project_tree_state.changed_files;
        let current = files.selected_index();
        let next = if is_up {
            current.saturating_sub(1)
        } else {
            (current + 1).min(files.len().saturating_sub(1))
        };
        return Some(AppAction::ProjectTree(
            ProjectTreeAction::SelectChangedFile(next),
        ));
    }
    if hit.rect == layout.diff_panel {
        return Some(AppAction::Navigation(if is_up {
            NavigationAction::ScrollDiffUp
        } else {
            NavigationAction::ScrollDiffDown
        }));
    }

    None
}

fn handle_staging_mouse_scroll(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
    is_up: bool,
) -> Option<AppAction> {
    use crate::ui::staging_layout::build_staging_layout;

    let layout = build_staging_layout(state.screen_area);

    if hit.rect == layout.unstaged_panel || hit.rect == layout.staged_panel {
        let (selection, unstaged) = if hit.rect == layout.unstaged_panel {
            (&state.staging_state.unstaged, true)
        } else {
            (&state.staging_state.staged, false)
        };
        let index = if is_up {
            selection.selected_index().saturating_sub(1)
        } else {
            selection
                .selected_index()
                .saturating_add(1)
                .min(selection.len().saturating_sub(1))
        };
        return Some(AppAction::Staging(if unstaged {
            StagingAction::SelectUnstaged(index)
        } else {
            StagingAction::SelectStaged(index)
        }));
    }

    if hit.rect == layout.diff_panel {
        return Some(if is_up {
            AppAction::Navigation(NavigationAction::ScrollDiffUp)
        } else {
            AppAction::Navigation(NavigationAction::ScrollDiffDown)
        });
    }

    None
}

/// Gère le scroll en utilisant le focus actuel.
fn handle_scroll_with_current_focus(state: &AppState, is_up: bool) -> Option<AppAction> {
    match state.view_mode {
        ViewMode::Graph => match state.focus {
            FocusPanel::BottomLeft => {
                if is_up {
                    Some(AppAction::Navigation(NavigationAction::FileUp))
                } else {
                    Some(AppAction::Navigation(NavigationAction::FileDown))
                }
            }
            FocusPanel::BottomRight => {
                if is_up {
                    Some(AppAction::Navigation(NavigationAction::ScrollDiffUp))
                } else {
                    Some(AppAction::Navigation(NavigationAction::ScrollDiffDown))
                }
            }
            _ => {
                if is_up {
                    Some(AppAction::Navigation(NavigationAction::MoveUp))
                } else {
                    Some(AppAction::Navigation(NavigationAction::MoveDown))
                }
            }
        },
        ViewMode::Staging => {
            if is_up {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            } else {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
        }
        ViewMode::Branches => {
            if is_up {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            } else {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
        }
        ViewMode::ProjectTree => {
            if is_up {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            } else {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
        }
        ViewMode::Blame => {
            if is_up {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            } else {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
        }
        _ => None,
    }
}
