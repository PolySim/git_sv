use crossterm::event::{MouseEvent, MouseEventKind};

use crate::state::action::{BranchAction, NavigationAction, StagingAction};
use crate::state::{AppAction, AppState, FocusPanel, ViewMode};

pub(crate) fn map_mouse(mouse: MouseEvent, state: &AppState) -> Option<AppAction> {
    use crate::ui::hit_test::hit_test;

    let x = mouse.column;
    let y = mouse.row;

    // Détecter la zone cliquée
    let hit_result = hit_test(state, x, y)?;

    match mouse.kind {
        MouseEventKind::Down(_) => {
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
    use crate::ui::hit_test::{
        calculate_commit_index, calculate_file_index, calculate_nav_tab, ClickableZone,
    };

    if state.view_mode == ViewMode::Staging {
        return handle_staging_mouse_click(state, &hit);
    }

    if state.view_mode == ViewMode::Branches {
        return handle_branches_mouse_click(state, &hit);
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
            let graph_height = state.graph_view.len();
            let selected_index = state.graph_view.selected_index();

            // Calculer l'offset de scroll visible (approximation)
            let visible_height = hit.rect.height.saturating_sub(2) as usize;
            let visible_commits = (visible_height / 2).max(1);
            let scroll_offset = selected_index.saturating_sub(visible_commits / 2);

            if let Some(commit_index) =
                calculate_commit_index(graph_height, scroll_offset, hit.relative_y)
            {
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

            if let Some(file_index) =
                calculate_file_index(file_count, hit.relative_y.saturating_sub(1))
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

fn handle_modal_click(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
) -> Option<AppAction> {
    use crate::ui::common::centered_rect;

    if state.ui.pending_confirmation.is_some() {
        let popup = centered_rect(60, 30, state.screen_area);
        if hit.relative_x >= popup.x && hit.relative_x < popup.x + popup.width {
            // Ligne des boutons approximative en bas du popup.
            let button_row = popup.y + popup.height.saturating_sub(2);
            if hit.relative_y == button_row {
                let rel_x = hit.relative_x.saturating_sub(popup.x);
                return if rel_x < popup.width / 3 {
                    Some(AppAction::ConfirmAction)
                } else {
                    Some(AppAction::CancelAction)
                };
            }
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
        let item_y = hit.relative_y.saturating_sub(1) as usize;
        if item_y < state.staging_state.unstaged_files().len() {
            return Some(AppAction::Staging(StagingAction::SelectUnstaged(item_y)));
        }
        return match state.staging_state.focus {
            StagingFocus::Unstaged => None,
            _ => Some(AppAction::Staging(StagingAction::FocusUnstaged)),
        };
    }

    if hit.rect == layout.staged_panel {
        let item_y = hit.relative_y.saturating_sub(1) as usize;
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
            (BranchesSection::Stashes, 0) => Some(AppAction::Branch(BranchAction::PrevSection)),
            (BranchesSection::Stashes, 1) => Some(AppAction::Branch(BranchAction::PrevSection)),
            (BranchesSection::Stashes, 2) => None,
            _ => None,
        };
    }

    if hit.rect == layout.list_panel {
        let item_y = hit.relative_y.saturating_sub(1) as usize;

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

fn handle_staging_mouse_scroll(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
    is_up: bool,
) -> Option<AppAction> {
    use crate::ui::staging_layout::build_staging_layout;

    let layout = build_staging_layout(state.screen_area);

    if hit.rect == layout.unstaged_panel || hit.rect == layout.staged_panel {
        return Some(if is_up {
            AppAction::Navigation(NavigationAction::MoveUp)
        } else {
            AppAction::Navigation(NavigationAction::MoveDown)
        });
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
