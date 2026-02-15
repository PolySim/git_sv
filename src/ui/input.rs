use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::time::Duration;

use crate::state::{
    AppAction, AppState, BranchesFocus, BranchesSection, FocusPanel, StagingFocus, ViewMode,
};

/// Poll un événement clavier et retourne l'action correspondante.
pub fn handle_input(state: &AppState) -> std::io::Result<Option<AppAction>> {
    handle_input_with_timeout(state, 100)
}

/// Poll un événement avec un timeout configurable (clavier + souris).
pub fn handle_input_with_timeout(
    state: &AppState,
    timeout_ms: u64,
) -> std::io::Result<Option<AppAction>> {
    if event::poll(Duration::from_millis(timeout_ms))? {
        match event::read()? {
            Event::Key(key) => Ok(map_key(key, state)),
            Event::Mouse(mouse) => Ok(map_mouse(mouse, state)),
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}

/// Mappe un événement clavier à une action de l'application.
fn map_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // Ctrl+C quitte toujours.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(AppAction::Quit);
    }

    // Si une confirmation est en attente, gérer y/n/ESC
    if state.pending_confirmation.is_some() {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(AppAction::ConfirmAction),
            KeyCode::Char('n') | KeyCode::Char('N') => Some(AppAction::CancelAction),
            KeyCode::Esc => Some(AppAction::CancelAction),
            _ => None,
        };
    }

    // Navigation entre les vues principales (toujours disponible)
    match key.code {
        KeyCode::Char('1') => return Some(AppAction::SwitchToGraph),
        KeyCode::Char('2') => return Some(AppAction::SwitchToStaging),
        KeyCode::Char('3') => return Some(AppAction::SwitchToBranches),
        _ => {}
    }

    // Si on est en mode Staging, utiliser les keybindings spécifiques
    if state.view_mode == ViewMode::Staging {
        return map_staging_key(key, state);
    }

    // Si on est en mode Branches, utiliser les keybindings spécifiques
    if state.view_mode == ViewMode::Branches {
        return map_branches_key(key, state);
    }

    // Ctrl+d / Ctrl+u pour page down/up
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('d') => {
                if state.focus == FocusPanel::Detail {
                    return Some(AppAction::DiffScrollDown);
                }
                return Some(AppAction::PageDown);
            }
            KeyCode::Char('u') => {
                if state.focus == FocusPanel::Detail {
                    return Some(AppAction::DiffScrollUp);
                }
                return Some(AppAction::PageUp);
            }
            _ => {}
        }
    }

    // Escape ferme l'overlay d'aide si actif.
    if key.code == KeyCode::Esc && state.view_mode == ViewMode::Help {
        return Some(AppAction::ToggleHelp);
    }

    // Escape pour revenir au panneau précédent quand on est dans Files ou Detail.
    if key.code == KeyCode::Esc {
        if state.focus == FocusPanel::Detail {
            return Some(AppAction::SwitchBottomMode);
        }
    }

    // Contexte: panneau de branches ouvert
    if state.show_branch_panel {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(AppAction::CloseBranchPanel),
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Enter => Some(AppAction::BranchCheckout),
            KeyCode::Char('n') => Some(AppAction::BranchCreate),
            KeyCode::Char('d') => Some(AppAction::BranchDelete),
            _ => None,
        };
    }

    // Navigation contextuelle selon le focus.
    match state.focus {
        FocusPanel::Files => {
            // Quand focus sur Files, j/k naviguent dans la liste des fichiers.
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return Some(AppAction::FileDown),
                KeyCode::Char('k') | KeyCode::Up => return Some(AppAction::FileUp),
                KeyCode::Enter => return Some(AppAction::SwitchBottomMode), // Passer au diff
                _ => {}
            }
        }
        FocusPanel::Detail => {
            // Quand focus sur Detail, j/k scrollent le diff.
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return Some(AppAction::DiffScrollDown),
                KeyCode::Char('k') | KeyCode::Up => return Some(AppAction::DiffScrollUp),
                _ => {}
            }
        }
        _ => {}
    }

    match key.code {
        // Navigation
        KeyCode::Char('q') => Some(AppAction::Quit),
        KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
        KeyCode::Char('g') | KeyCode::Home => Some(AppAction::GoTop),
        KeyCode::Char('G') | KeyCode::End => Some(AppAction::GoBottom),
        KeyCode::PageUp => Some(AppAction::PageUp),
        KeyCode::PageDown => Some(AppAction::PageDown),
        KeyCode::Enter => Some(AppAction::Select),

        // Actions git
        KeyCode::Char('c') => Some(AppAction::CommitPrompt),
        KeyCode::Char('s') => Some(AppAction::StashPrompt),
        KeyCode::Char('m') => Some(AppAction::MergePrompt),
        KeyCode::Char('b') => Some(AppAction::BranchList),

        // Aide
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),

        // Rafraîchir
        KeyCode::Char('r') => Some(AppAction::Refresh),

        // Basculer entre les modes du panneau bas-gauche
        KeyCode::Tab => Some(AppAction::SwitchBottomMode),

        _ => None,
    }
}

/// Mappe les touches pour la vue branches.
fn map_branches_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // Si on est en mode Input.
    if state.branches_view_state.focus == BranchesFocus::Input {
        return match key.code {
            KeyCode::Enter => Some(AppAction::ConfirmInput),
            KeyCode::Esc => Some(AppAction::CancelInput),
            KeyCode::Char(c) => Some(AppAction::InsertChar(c)),
            KeyCode::Backspace => Some(AppAction::DeleteChar),
            KeyCode::Left => Some(AppAction::MoveCursorLeft),
            KeyCode::Right => Some(AppAction::MoveCursorRight),
            _ => None,
        };
    }

    // Navigation globale.
    match key.code {
        KeyCode::Char('1') => return Some(AppAction::SwitchToGraph),
        KeyCode::Char('2') => return Some(AppAction::SwitchToStaging),
        KeyCode::Tab => return Some(AppAction::NextSection),
        KeyCode::BackTab => return Some(AppAction::PrevSection),
        KeyCode::Char('q') => return Some(AppAction::Quit),
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp),
        _ => {}
    }

    // Actions par section.
    match state.branches_view_state.section {
        BranchesSection::Branches => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Enter => Some(AppAction::BranchCheckout),
            KeyCode::Char('n') => Some(AppAction::BranchCreate),
            KeyCode::Char('d') => Some(AppAction::BranchDelete),
            KeyCode::Char('r') => Some(AppAction::BranchRename),
            KeyCode::Char('R') => Some(AppAction::ToggleRemoteBranches),
            _ => None,
        },
        BranchesSection::Worktrees => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('n') => Some(AppAction::WorktreeCreate),
            KeyCode::Char('d') => Some(AppAction::WorktreeRemove),
            _ => None,
        },
        BranchesSection::Stashes => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('a') => Some(AppAction::StashApply),
            KeyCode::Char('p') => Some(AppAction::StashPop),
            KeyCode::Char('d') => Some(AppAction::StashDrop),
            KeyCode::Char('s') => Some(AppAction::StashSave),
            _ => None,
        },
    }
}

/// Mappe les touches pour la vue staging.
fn map_staging_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // Touches globales de la vue staging
    match key.code {
        KeyCode::Char('q') => return Some(AppAction::Quit),
        KeyCode::Char('r') => return Some(AppAction::Refresh),
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp),
        _ => {}
    }

    // Navigation selon le focus dans la vue staging
    match state.staging_state.focus {
        StagingFocus::Unstaged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('s') | KeyCode::Enter => Some(AppAction::StageFile),
            KeyCode::Char('a') => Some(AppAction::StageAll),
            KeyCode::Tab => Some(AppAction::SwitchStagingFocus),
            KeyCode::Char('c') => Some(AppAction::StartCommitMessage),
            _ => None,
        },
        StagingFocus::Staged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('u') | KeyCode::Enter => Some(AppAction::UnstageFile),
            KeyCode::Char('U') => Some(AppAction::UnstageAll),
            KeyCode::Tab => Some(AppAction::SwitchStagingFocus),
            KeyCode::Char('c') => Some(AppAction::StartCommitMessage),
            _ => None,
        },
        StagingFocus::Diff => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::DiffScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::DiffScrollUp),
            KeyCode::Tab | KeyCode::Esc => Some(AppAction::SwitchStagingFocus),
            KeyCode::Char('c') => Some(AppAction::StartCommitMessage),
            _ => None,
        },
        StagingFocus::CommitMessage => match key.code {
            KeyCode::Enter => Some(AppAction::ConfirmCommit),
            KeyCode::Esc => Some(AppAction::CancelCommitMessage),
            KeyCode::Char(c) => Some(AppAction::InsertChar(c)),
            KeyCode::Backspace => Some(AppAction::DeleteChar),
            KeyCode::Left => Some(AppAction::MoveCursorLeft),
            KeyCode::Right => Some(AppAction::MoveCursorRight),
            _ => None,
        },
    }
}

/// Mappe un événement souris à une action de l'application.
fn map_mouse(mouse: MouseEvent, state: &AppState) -> Option<AppAction> {
    // Ignorer les événements de souris si une confirmation est en attente
    if state.pending_confirmation.is_some() {
        return None;
    }

    match mouse.kind {
        MouseEventKind::Down(_) => {
            // Pour l'instant, le clic sélectionne simplement (sera amélioré avec hit-testing)
            // On pourrait ajouter ici la logique pour déterminer quel élément a été cliqué
            // en fonction de la position (mouse.row, mouse.column)
            None
        }
        MouseEventKind::ScrollUp => {
            // Scroll up dans le panneau actif
            match state.view_mode {
                ViewMode::Graph => {
                    if state.focus == FocusPanel::Files {
                        Some(AppAction::FileUp)
                    } else if state.focus == FocusPanel::Detail {
                        Some(AppAction::DiffScrollUp)
                    } else {
                        Some(AppAction::MoveUp)
                    }
                }
                ViewMode::Staging => Some(AppAction::MoveUp),
                ViewMode::Branches => Some(AppAction::MoveUp),
                _ => None,
            }
        }
        MouseEventKind::ScrollDown => {
            // Scroll down dans le panneau actif
            match state.view_mode {
                ViewMode::Graph => {
                    if state.focus == FocusPanel::Files {
                        Some(AppAction::FileDown)
                    } else if state.focus == FocusPanel::Detail {
                        Some(AppAction::DiffScrollDown)
                    } else {
                        Some(AppAction::MoveDown)
                    }
                }
                ViewMode::Staging => Some(AppAction::MoveDown),
                ViewMode::Branches => Some(AppAction::MoveDown),
                _ => None,
            }
        }
        _ => None,
    }
}
