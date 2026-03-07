//! Mapping clavier et souris vers les actions de l'application.
//!
//! Ce module est le point central de gestion des entrées utilisateur.
//! Il traduit les événements crossterm en [`AppAction`] selon le mode
//! de vue actif et l'état courant.
//!
//! # Architecture
//!
//! La fonction principale `map_key()` applique les règles dans l'ordre de priorité :
//!
//! 1. **Ctrl+C** — quitter (toujours prioritaire)
//! 2. **Merge picker actif** — keybindings dédiés au picker
//! 3. **Confirmation en attente** — y/n/Esc uniquement
//! 4. **Recherche active** — saisie dans la barre de recherche
//! 5. **Popup de filtre ouvert** — saisie dans le filtre
//! 6. **Mode Staging / focus CommitMessage** — saisie du message de commit
//! 7. **Mode Branches / focus Input** — saisie du nom de branche
//! 8. **Changements de vue globaux** — touches `1`, `2`, `3`, `4`
//! 9. **Mode Conflicts** → `map_conflicts_key()`
//! 10. **Mode Staging** → `map_staging_key()`
//! 11. **Mode Branches** → `map_branches_key()`
//! 12. **Mode Blame** → `map_blame_key()`
//! 13. **Raccourcis globaux** (Ctrl+D/U, Esc, Tab, etc.)
//! 14. **Keybindings de la vue Graph** (défaut)

#![allow(dead_code)]

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::time::Duration;

use crate::state::action::{
    BranchAction, ConflictAction, EditAction, FilterAction, GitAction, NavigationAction,
    SearchAction, StagingAction,
};
use crate::state::{
    AppAction, AppState, BranchesFocus, BranchesSection, ConflictPanelFocus, FocusPanel,
    StagingFocus, ViewMode,
};

/// Timeout par défaut pour le polling des événements (ms).
const DEFAULT_INPUT_TIMEOUT_MS: u64 = 100;

/// Poll un événement clavier et retourne l'action correspondante.
pub fn handle_input(state: &AppState) -> std::io::Result<Option<AppAction>> {
    handle_input_with_timeout(state, DEFAULT_INPUT_TIMEOUT_MS)
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

    // Si le merge picker est actif, gérer ses keybindings
    if state.merge_picker.as_ref().map_or(false, |p| p.is_active) {
        return match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MergePickerDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MergePickerUp),
            KeyCode::Enter => Some(AppAction::MergePickerConfirm),
            KeyCode::Esc => Some(AppAction::MergePickerCancel),
            _ => None,
        };
    }

    // Si le reset picker est actif, gérer ses keybindings
    if state.reset_picker.as_ref().map_or(false, |p| p.is_active) {
        return match key.code {
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('s') => {
                Some(AppAction::ResetPickerSelectSoft)
            }
            KeyCode::Char('k') | KeyCode::Up | KeyCode::Char('h') => {
                Some(AppAction::ResetPickerSelectHard)
            }
            KeyCode::Enter => Some(AppAction::ResetPickerConfirm),
            KeyCode::Esc => Some(AppAction::ResetPickerCancel),
            _ => None,
        };
    }

    // Si une confirmation est en attente, gérer y/n/ESC
    if state.pending_confirmation.is_some() {
        return match key.code {
            KeyCode::Char('y' | 'Y') => Some(AppAction::ConfirmAction),
            KeyCode::Char('n' | 'N') => Some(AppAction::CancelAction),
            KeyCode::Esc => Some(AppAction::CancelAction),
            _ => None,
        };
    }

    // Si la recherche est active, gérer les inputs de recherche
    if state.search_state.is_active {
        return match key.code {
            KeyCode::Esc => Some(AppAction::Search(SearchAction::Close)),
            KeyCode::Enter => Some(AppAction::Search(SearchAction::Execute)),
            KeyCode::Down => Some(AppAction::Search(SearchAction::NextResult)),
            KeyCode::Up => Some(AppAction::Search(SearchAction::PreviousResult)),
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppAction::Search(SearchAction::NextResult))
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppAction::Search(SearchAction::PreviousResult))
            }
            KeyCode::Tab => Some(AppAction::Search(SearchAction::ChangeType)),
            KeyCode::Char(c) => Some(AppAction::Search(SearchAction::InsertChar(c))),
            KeyCode::Backspace => Some(AppAction::Search(SearchAction::DeleteChar)),
            _ => None,
        };
    }

    // Si le popup de filtre est ouvert, gérer ses inputs
    if state.filter_popup.is_open {
        return match key.code {
            KeyCode::Esc => Some(AppAction::Filter(FilterAction::Close)),
            KeyCode::Enter => Some(AppAction::Filter(FilterAction::Apply)),
            KeyCode::Tab | KeyCode::Down => Some(AppAction::Filter(FilterAction::NextField)),
            KeyCode::BackTab | KeyCode::Up => Some(AppAction::Filter(FilterAction::PreviousField)),
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppAction::Filter(FilterAction::Clear))
            }
            KeyCode::Char(c) => Some(AppAction::Filter(FilterAction::InsertChar(c))),
            KeyCode::Backspace => Some(AppAction::Filter(FilterAction::DeleteChar)),
            _ => None,
        };
    }

    // Si on est en mode Staging avec focus sur CommitMessage, dispatcher immédiatement
    // sans intercepter les raccourcis globaux (permet de taper "1", "2", "3" dans le message)
    if state.view_mode == ViewMode::Staging
        && state.staging_state.focus == StagingFocus::CommitMessage
    {
        return map_staging_key(key, state);
    }

    // Si on est en mode Branches avec focus sur Input, dispatcher immédiatement
    // sans intercepter les raccourcis globaux (permet de taper "1", "2", "3" dans le nom)
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.focus == BranchesFocus::Input
    {
        return map_branches_key(key, state);
    }

    // Navigation entre les vues principales (toujours disponible, sauf en mode saisie)
    match key.code {
        KeyCode::Char('1') => return Some(AppAction::SwitchView(ViewMode::Graph)),
        KeyCode::Char('2') => return Some(AppAction::SwitchView(ViewMode::Staging)),
        KeyCode::Char('3') => return Some(AppAction::SwitchView(ViewMode::Branches)),
        KeyCode::Char('4') => {
            if state.conflicts_state.is_some() {
                return Some(AppAction::SwitchView(ViewMode::Conflicts));
            }
        }
        _ => {}
    }

    // Si on est en mode Conflicts, utiliser les keybindings spécifiques
    if state.view_mode == ViewMode::Conflicts {
        return map_conflicts_key(key, state);
    }

    // Si on est en mode Staging, utiliser les keybindings spécifiques
    if state.view_mode == ViewMode::Staging {
        return map_staging_key(key, state);
    }

    // Si on est en mode Branches, utiliser les keybindings spécifiques
    if state.view_mode == ViewMode::Branches {
        return map_branches_key(key, state);
    }

    // Si on est en mode Blame, utiliser les keybindings spécifiques
    if state.view_mode == ViewMode::Blame {
        return map_blame_key(key, state);
    }

    // Ctrl+d / Ctrl+u pour page down/up
    // Ctrl+R pour effacer les filtres si actifs
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('p') => {
                return Some(AppAction::Git(GitAction::ForcePush));
            }
            KeyCode::Char('d') => {
                if state.focus == FocusPanel::BottomRight {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffPageDown));
                }
                return Some(AppAction::Navigation(NavigationAction::PageDown));
            }
            KeyCode::Char('u') => {
                if state.focus == FocusPanel::BottomRight {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffPageUp));
                }
                return Some(AppAction::Navigation(NavigationAction::PageUp));
            }
            KeyCode::Char('r') => {
                if state.graph_filter.is_active() {
                    return Some(AppAction::Filter(FilterAction::Clear));
                }
            }
            _ => {}
        }
    }

    // Escape ferme l'overlay d'aide si actif.
    if key.code == KeyCode::Esc && state.view_mode == ViewMode::Help {
        return Some(AppAction::ToggleHelp);
    }

    // Escape pour quitter le mode diff plein écran ou revenir au panneau précédent.
    if key.code == KeyCode::Esc {
        // Si mode diff plein écran actif, le quitter
        if state.graph_view.diff_fullscreen {
            return Some(AppAction::ToggleDiffFullscreen);
        }
        match state.focus {
            FocusPanel::BottomRight => {
                return Some(AppAction::SwitchBottomMode);
            }
            FocusPanel::BottomLeft => {
                // Retourner au focus Graph depuis le panneau fichiers
                return Some(AppAction::Navigation(
                    crate::state::action::NavigationAction::BackToGraph,
                ));
            }
            _ => {}
        }
    }

    // Contexte: panneau de branches ouvert
    if state.show_branch_panel {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('b') => Some(AppAction::CloseBranchPanel),
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Enter => Some(AppAction::Branch(BranchAction::Checkout)),
            KeyCode::Char('n') => Some(AppAction::Branch(BranchAction::Create)),
            KeyCode::Char('d') => Some(AppAction::Branch(BranchAction::Delete)),
            _ => None,
        };
    }

    // Navigation contextuelle selon le focus.
    match state.focus {
        FocusPanel::BottomLeft => {
            // Quand focus sur BottomLeft, j/k naviguent dans la liste des fichiers.
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    return Some(AppAction::Navigation(NavigationAction::FileDown))
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    return Some(AppAction::Navigation(NavigationAction::FileUp))
                }
                KeyCode::Char(' ') => return Some(AppAction::Select),
                KeyCode::Char('z') | KeyCode::Enter => {
                    return Some(AppAction::ToggleDiffFullscreen)
                }
                _ => {}
            }
        }
        FocusPanel::BottomRight => {
            // Quand focus sur BottomRight, j/k scrollent le diff.
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffDown))
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffUp))
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffLeft))
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffRight))
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffTop))
                }
                KeyCode::Char('G') | KeyCode::End => {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffBottom))
                }
                KeyCode::Char('z') | KeyCode::Enter => {
                    return Some(AppAction::ToggleDiffFullscreen)
                }
                KeyCode::Char('v') => return Some(AppAction::ToggleDiffViewMode),
                KeyCode::PageUp => {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffPageUp))
                }
                KeyCode::PageDown => {
                    return Some(AppAction::Navigation(NavigationAction::ScrollDiffPageDown))
                }
                _ => {}
            }
        }
        _ => {}
    }

    // ───────────────────────────────────────────────
    // Vue Graph (keybindings par défaut)
    // ───────────────────────────────────────────────
    match key.code {
        // Navigation dans le graphe
        KeyCode::Char('q') => Some(AppAction::Quit),
        KeyCode::Char('j') | KeyCode::Down => {
            Some(AppAction::Navigation(NavigationAction::MoveDown))
        }
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::Navigation(NavigationAction::MoveUp)),
        KeyCode::Char('g') | KeyCode::Home => Some(AppAction::Navigation(NavigationAction::GoTop)),
        KeyCode::Char('G') | KeyCode::End => {
            Some(AppAction::Navigation(NavigationAction::GoBottom))
        }
        KeyCode::PageUp => Some(AppAction::Navigation(NavigationAction::PageUp)),
        KeyCode::PageDown => Some(AppAction::Navigation(NavigationAction::PageDown)),
        KeyCode::Enter => Some(AppAction::Select),

        // Actions git
        KeyCode::Char('c') => Some(AppAction::Git(GitAction::CommitPrompt)),
        KeyCode::Char('s') => Some(AppAction::Git(GitAction::StashPrompt)),
        KeyCode::Char('m') => Some(AppAction::Git(GitAction::MergePrompt)),
        KeyCode::Char('b') => Some(AppAction::Branch(BranchAction::List)),
        KeyCode::Char('P') => Some(AppAction::Git(GitAction::Push)),
        KeyCode::Char('p') => Some(AppAction::Git(GitAction::Pull)),
        KeyCode::Char('f') => Some(AppAction::Git(GitAction::Fetch)),

        // Recherche
        KeyCode::Char('/') => Some(AppAction::Search(SearchAction::Open)),
        KeyCode::Char('n') => Some(AppAction::Search(SearchAction::NextResult)),
        KeyCode::Char('N') => Some(AppAction::Search(SearchAction::PreviousResult)),

        // Filtre
        KeyCode::Char('F') => Some(AppAction::Filter(FilterAction::Open)),

        // Vue blame
        KeyCode::Char('B') => Some(AppAction::Git(GitAction::OpenBlame)),

        // Cherry-pick
        KeyCode::Char('x') => Some(AppAction::Git(GitAction::CherryPick)),

        // Reset
        KeyCode::Char('R') => Some(AppAction::Git(GitAction::ResetPrompt)),

        // Abort merge (uniquement si un merge est en cours)
        KeyCode::Char('A') if state.is_merging => Some(AppAction::Git(GitAction::AbortMerge)),

        // Charger plus d'historique (pagination)
        KeyCode::Char('L') => Some(AppAction::LoadMoreHistory),

        // Aide
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),

        // Rafraîchir
        KeyCode::Char('r') => Some(AppAction::Refresh),

        // Copier le contenu du panneau actif dans le clipboard
        KeyCode::Char('y') => Some(AppAction::CopyPanelContent),

        // Changer le focus entre les panneaux principaux (Graph <-> BottomLeft)
        KeyCode::Tab => Some(AppAction::Navigation(
            crate::state::action::NavigationAction::SwitchPanel,
        )),

        // Basculer entre les modes du panneau bas-gauche (Files / Parents)
        KeyCode::Char('M') => Some(AppAction::SwitchBottomMode),

        _ => None,
    }
}

/// Mappe les touches pour la vue branches.
///
/// # Priorités
/// 1. Focus Input (saisie de nom) — capture toutes les touches
/// 2. Raccourcis globaux (1/2/3, q, y, ?, P)
/// 3. Actions spécifiques à la section active (Branches/Worktrees/Stashes)
fn map_branches_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // Si on est en mode Input.
    if state.branches_view_state.focus == BranchesFocus::Input {
        return match key.code {
            KeyCode::Enter => Some(AppAction::Branch(BranchAction::ConfirmInput)),
            KeyCode::Esc => Some(AppAction::Branch(BranchAction::CancelInput)),
            KeyCode::Char(c) => Some(AppAction::Edit(EditAction::InsertChar(c))),
            KeyCode::Backspace => Some(AppAction::Edit(EditAction::DeleteCharBefore)),
            KeyCode::Left => Some(AppAction::Edit(EditAction::CursorLeft)),
            KeyCode::Right => Some(AppAction::Edit(EditAction::CursorRight)),
            _ => None,
        };
    }

    // Navigation globale.
    match key.code {
        KeyCode::Char('1') => return Some(AppAction::SwitchView(ViewMode::Graph)),
        KeyCode::Char('2') => return Some(AppAction::SwitchView(ViewMode::Staging)),
        KeyCode::Tab => return Some(AppAction::Branch(BranchAction::NextSection)),
        KeyCode::BackTab => return Some(AppAction::Branch(BranchAction::PrevSection)),
        KeyCode::Char('q') => return Some(AppAction::Quit),
        KeyCode::Char('y') => return Some(AppAction::CopyPanelContent),
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp),
        KeyCode::Char('P') => return Some(AppAction::Git(GitAction::Push)),
        _ => {}
    }

    // Actions par section.
    match state.branches_view_state.section {
        BranchesSection::Branches => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Enter => Some(AppAction::Branch(BranchAction::Checkout)),
            KeyCode::Char('n') => Some(AppAction::Branch(BranchAction::Create)),
            KeyCode::Char('d') => Some(AppAction::Branch(BranchAction::Delete)),
            KeyCode::Char('r') => Some(AppAction::Branch(BranchAction::Rename)),
            KeyCode::Char('R') => Some(AppAction::Branch(BranchAction::ToggleRemote)),
            KeyCode::Char('m') => Some(AppAction::Git(GitAction::MergePrompt)),
            _ => None,
        },
        BranchesSection::Worktrees => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Char('n') => Some(AppAction::Branch(BranchAction::WorktreeCreate)),
            KeyCode::Char('d') => Some(AppAction::Branch(BranchAction::WorktreeRemove)),
            _ => None,
        },
        BranchesSection::Stashes => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Char('l') | KeyCode::Right => {
                Some(AppAction::Branch(BranchAction::StashFileNext))
            }
            KeyCode::Char('h') | KeyCode::Left => {
                Some(AppAction::Branch(BranchAction::StashFilePrev))
            }
            KeyCode::Char('a') => Some(AppAction::Branch(BranchAction::StashApply)),
            KeyCode::Char('p') => Some(AppAction::Branch(BranchAction::StashPop)),
            KeyCode::Char('d') => Some(AppAction::Branch(BranchAction::StashDrop)),
            KeyCode::Char('s') => Some(AppAction::Branch(BranchAction::StashSave)),
            KeyCode::Char('J') => {
                Some(AppAction::Navigation(NavigationAction::ScrollStashDiffDown))
            }
            KeyCode::Char('K') => Some(AppAction::Navigation(NavigationAction::ScrollStashDiffUp)),
            _ => None,
        },
    }
}

/// Mappe les touches pour la vue staging.
///
/// # Priorités
/// 1. Focus CommitMessage — capture toutes les touches pour la saisie
/// 2. Raccourcis globaux (q, r, y, ?, P)
/// 3. Actions par focus (Unstaged/Staged/Diff)
fn map_staging_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // Vérifier d'abord si on est en mode saisie de commit
    if state.staging_state.focus == StagingFocus::CommitMessage {
        return match key.code {
            KeyCode::Enter => Some(AppAction::Staging(StagingAction::ConfirmCommit)),
            KeyCode::Esc => Some(AppAction::Staging(StagingAction::CancelCommit)),
            KeyCode::Char(c) => Some(AppAction::Edit(EditAction::InsertChar(c))),
            KeyCode::Backspace => Some(AppAction::Edit(EditAction::DeleteCharBefore)),
            KeyCode::Left => Some(AppAction::Edit(EditAction::CursorLeft)),
            KeyCode::Right => Some(AppAction::Edit(EditAction::CursorRight)),
            _ => None,
        };
    }

    // Touches globales de la vue staging
    match key.code {
        KeyCode::Char('q') => return Some(AppAction::Quit),
        KeyCode::Char('r') => return Some(AppAction::Refresh),
        KeyCode::Char('y') => return Some(AppAction::CopyPanelContent),
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp),
        KeyCode::Char('P') => return Some(AppAction::Git(GitAction::Push)),
        KeyCode::Char('A') if state.is_merging => {
            return Some(AppAction::Git(GitAction::AbortMerge))
        }
        _ => {}
    }

    // Navigation selon le focus dans la vue staging
    match state.staging_state.focus {
        StagingFocus::Unstaged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Char(' ') => Some(AppAction::Staging(StagingAction::FocusDiff)),
            KeyCode::Char('s') | KeyCode::Enter => {
                Some(AppAction::Staging(StagingAction::StageFile))
            }
            KeyCode::Char('S') => Some(AppAction::Staging(StagingAction::StashSelectedFile)),
            KeyCode::Char('a') => Some(AppAction::Staging(StagingAction::StageAll)),
            KeyCode::Char('d') => Some(AppAction::Staging(StagingAction::DiscardFile)),
            KeyCode::Char('D') => Some(AppAction::Staging(StagingAction::DiscardAll)),
            KeyCode::Tab => Some(AppAction::Staging(StagingAction::SwitchFocus)),
            KeyCode::Char('c') => Some(AppAction::Staging(StagingAction::StartCommitMessage)),
            KeyCode::Char('A') if !state.is_merging => Some(AppAction::Git(GitAction::AmendCommit)),
            _ if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('S') =>
            {
                Some(AppAction::Staging(StagingAction::StashUnstagedFiles))
            }
            _ => None,
        },
        StagingFocus::Staged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::MoveDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::MoveUp))
            }
            KeyCode::Char(' ') => Some(AppAction::Staging(StagingAction::FocusDiff)),
            KeyCode::Char('u') | KeyCode::Enter => {
                Some(AppAction::Staging(StagingAction::UnstageFile))
            }
            KeyCode::Char('U') => Some(AppAction::Staging(StagingAction::UnstageAll)),
            KeyCode::Tab => Some(AppAction::Staging(StagingAction::SwitchFocus)),
            KeyCode::Char('c') => Some(AppAction::Staging(StagingAction::StartCommitMessage)),
            KeyCode::Char('A') if !state.is_merging => Some(AppAction::Git(GitAction::AmendCommit)),
            _ => None,
        },
        StagingFocus::Diff => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffDown))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffUp))
            }
            KeyCode::Char('h') | KeyCode::Left => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffLeft))
            }
            KeyCode::Char('l') | KeyCode::Right => {
                Some(AppAction::Navigation(NavigationAction::ScrollDiffRight))
            }
            KeyCode::Tab | KeyCode::Esc => Some(AppAction::Staging(StagingAction::SwitchFocus)),
            KeyCode::Char('c') => Some(AppAction::Staging(StagingAction::StartCommitMessage)),
            KeyCode::Char('A') if !state.is_merging => Some(AppAction::Git(GitAction::AmendCommit)),
            KeyCode::Char('v') => Some(AppAction::ToggleDiffViewMode),
            _ => None,
        },
        // StagingFocus::CommitMessage est géré en priorité au début de la fonction
        StagingFocus::CommitMessage => unreachable!(),
    }
}

/// Mappe les keybindings pour la vue Blame.
fn map_blame_key(key: KeyEvent, _state: &AppState) -> Option<AppAction> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(AppAction::Git(GitAction::CloseBlame)),
        KeyCode::Char('j') | KeyCode::Down => {
            Some(AppAction::Navigation(NavigationAction::MoveDown))
        }
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::Navigation(NavigationAction::MoveUp)),
        KeyCode::Char('g') | KeyCode::Home => Some(AppAction::Navigation(NavigationAction::GoTop)),
        KeyCode::Char('G') | KeyCode::End => {
            Some(AppAction::Navigation(NavigationAction::GoBottom))
        }
        KeyCode::PageUp => Some(AppAction::Navigation(NavigationAction::PageUp)),
        KeyCode::PageDown => Some(AppAction::Navigation(NavigationAction::PageDown)),
        KeyCode::Enter => Some(AppAction::Git(GitAction::JumpToBlameCommit)),
        KeyCode::Char('y') => Some(AppAction::CopyPanelContent),
        _ => None,
    }
}

/// Mappe les keybindings pour la vue de résolution de conflits.
fn map_conflicts_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    use crate::git::conflict::ConflictResolutionMode;

    // Si une confirmation est en attente (pour ConflictValidateMerge)
    if state.pending_confirmation.is_some() {
        return match key.code {
            KeyCode::Char('y' | 'Y') => Some(AppAction::ConfirmAction),
            KeyCode::Char('n' | 'N') => Some(AppAction::CancelAction),
            KeyCode::Esc => Some(AppAction::CancelAction),
            _ => None,
        };
    }

    // Récupérer le panneau actif, le mode de résolution et l'état d'édition
    let conflicts_state = state.conflicts_state.as_ref();
    let panel_focus = conflicts_state.map(|s| s.panel_focus);
    let is_editing = conflicts_state.map_or(false, |s| s.is_editing);
    let resolution_mode =
        conflicts_state.map_or(ConflictResolutionMode::Block, |s| s.resolution_mode);

    // Si en mode édition dans le panneau résultat, capturer toutes les touches comme du texte
    if is_editing {
        return match key.code {
            KeyCode::Esc => Some(AppAction::Conflict(ConflictAction::StopEditing)),
            // Sauvegarder et quitter l'édition
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppAction::Conflict(ConflictAction::ConfirmEdit))
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppAction::Conflict(ConflictAction::ConfirmEdit))
            }
            KeyCode::Char(c) => Some(AppAction::Conflict(ConflictAction::EditInsertChar(c))),
            KeyCode::Backspace => Some(AppAction::Conflict(ConflictAction::EditBackspace)),
            KeyCode::Delete => Some(AppAction::Conflict(ConflictAction::EditDelete)),
            KeyCode::Enter => Some(AppAction::Conflict(ConflictAction::EditNewline)),
            KeyCode::Up => Some(AppAction::Conflict(ConflictAction::EditCursorUp)),
            KeyCode::Down => Some(AppAction::Conflict(ConflictAction::EditCursorDown)),
            KeyCode::Left => Some(AppAction::Conflict(ConflictAction::EditCursorLeft)),
            KeyCode::Right => Some(AppAction::Conflict(ConflictAction::EditCursorRight)),
            _ => None,
        };
    }

    match key.code {
        // Tab et Shift+Tab : basculer entre les panneaux
        KeyCode::Tab => Some(AppAction::Conflict(ConflictAction::SwitchPanel)),
        KeyCode::BackTab => Some(AppAction::Conflict(ConflictAction::SwitchPanel)),

        // Navigation flèches/j/k : dépend du panneau actif et du mode de résolution
        KeyCode::Char('j') | KeyCode::Down => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::NextFile))
            }
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    // En mode Fichier, naviguer entre les fichiers (pas entre sections)
                    ConflictResolutionMode::File => {
                        Some(AppAction::Conflict(ConflictAction::NextFile))
                    }
                    ConflictResolutionMode::Line => {
                        Some(AppAction::Conflict(ConflictAction::LineDown))
                    }
                    ConflictResolutionMode::Block => {
                        Some(AppAction::Conflict(ConflictAction::NextSection))
                    }
                }
            }
            Some(ConflictPanelFocus::ResultPanel) => {
                Some(AppAction::Conflict(ConflictAction::ResultScrollDown))
            }
            _ => None,
        },
        KeyCode::Char('k') | KeyCode::Up => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::PreviousFile))
            }
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    // En mode Fichier, naviguer entre les fichiers (pas entre sections)
                    ConflictResolutionMode::File => {
                        Some(AppAction::Conflict(ConflictAction::PreviousFile))
                    }
                    ConflictResolutionMode::Line => {
                        Some(AppAction::Conflict(ConflictAction::LineUp))
                    }
                    ConflictResolutionMode::Block => {
                        Some(AppAction::Conflict(ConflictAction::PreviousSection))
                    }
                }
            }
            Some(ConflictPanelFocus::ResultPanel) => {
                Some(AppAction::Conflict(ConflictAction::ResultScrollUp))
            }
            _ => None,
        },

        // Résolution rapide depuis le panneau FileList
        KeyCode::Char('o') | KeyCode::Left => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::AcceptOursFile))
            }
            _ => None,
        },
        KeyCode::Char('t') | KeyCode::Right => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::AcceptTheirsFile))
            }
            _ => None,
        },
        // Marquer comme résolu depuis le panneau FileList
        KeyCode::Char('r') => match panel_focus {
            Some(ConflictPanelFocus::FileList) => {
                Some(AppAction::Conflict(ConflictAction::MarkResolved))
            }
            _ => None,
        },

        // Résolution "Both" uniquement en mode Bloc (depuis les panneaux Ours/Theirs)
        KeyCode::Char('b') => {
            if matches!(
                panel_focus,
                Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel)
            ) && resolution_mode == ConflictResolutionMode::Block
            {
                Some(AppAction::Conflict(ConflictAction::AcceptBoth))
            } else {
                None
            }
        }

        // Mode édition (panneau résultat uniquement)
        KeyCode::Char('i' | 'e') => {
            if panel_focus == Some(ConflictPanelFocus::ResultPanel) {
                Some(AppAction::Conflict(ConflictAction::StartEditing))
            } else {
                None
            }
        }

        // Changement de mode de résolution (mapping direct)
        KeyCode::Char('F') => Some(AppAction::Conflict(ConflictAction::SetModeFile)),
        KeyCode::Char('B') => Some(AppAction::Conflict(ConflictAction::SetModeBlock)),
        KeyCode::Char('L') => Some(AppAction::Conflict(ConflictAction::SetModeLine)),

        // Espace: toggle la sélection en mode Block ou Ligne
        KeyCode::Char(' ') => match panel_focus {
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    ConflictResolutionMode::Block => {
                        Some(AppAction::Conflict(ConflictAction::EnterResolve))
                    }
                    ConflictResolutionMode::Line => {
                        Some(AppAction::Conflict(ConflictAction::ToggleLine))
                    }
                    _ => None,
                }
            }
            _ => None,
        },

        // Enter: validation contextuelle selon le panneau et le mode
        KeyCode::Enter => match panel_focus {
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    // En mode Fichier, Enter résout selon le panneau actif
                    ConflictResolutionMode::File => {
                        Some(AppAction::Conflict(ConflictAction::EnterResolve))
                    }
                    // En mode Block/Ligne, Enter valide le fichier (écrit sur disque)
                    ConflictResolutionMode::Block | ConflictResolutionMode::Line => {
                        Some(AppAction::Conflict(ConflictAction::MarkResolved))
                    }
                }
            }
            _ => None,
        },
        KeyCode::Char('V') => Some(AppAction::Conflict(ConflictAction::FinalizeMerge)),
        KeyCode::Char('q') | KeyCode::Esc => {
            if is_editing {
                Some(AppAction::Conflict(ConflictAction::StopEditing))
            } else {
                Some(AppAction::Conflict(ConflictAction::LeaveView))
            }
        }
        KeyCode::Char('A') => Some(AppAction::Conflict(ConflictAction::AbortMerge)),

        // Vues
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),

        // Navigation entre vues
        KeyCode::Char('1') => return Some(AppAction::SwitchView(ViewMode::Graph)),
        KeyCode::Char('2') => return Some(AppAction::SwitchView(ViewMode::Staging)),
        KeyCode::Char('3') => return Some(AppAction::SwitchView(ViewMode::Branches)),
        _ => None,
    }
}

/// Mappe un événement souris à une action de l'application.
fn map_mouse(mouse: MouseEvent, state: &AppState) -> Option<AppAction> {
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
        ClickableZone::BranchPanel => handle_branch_panel_click(state, &hit),
        ClickableZone::NavBar => {
            // Clic sur un tab de navigation
            if let Some(view_mode) = calculate_nav_tab(hit.relative_x) {
                Some(AppAction::SwitchView(view_mode))
            } else {
                None
            }
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

    if state.pending_confirmation.is_some() {
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

fn handle_branch_panel_click(
    state: &AppState,
    hit: &crate::ui::hit_test::HitTestResult,
) -> Option<AppAction> {
    use crate::ui::common::centered_rect;

    let popup = centered_rect(60, 50, state.screen_area);

    // Clic hors popup -> fermer.
    if hit.relative_x < popup.x
        || hit.relative_x >= popup.x + popup.width
        || hit.relative_y < popup.y
        || hit.relative_y >= popup.y + popup.height
    {
        return Some(AppAction::CloseBranchPanel);
    }

    // Zone liste : header + items.
    let item_y = hit.relative_y.saturating_sub(popup.y + 1) as usize;
    if item_y < state.branches.len() {
        let current = state.branch_selected as isize;
        let target = item_y as isize;
        if target > current {
            return Some(AppAction::Navigation(NavigationAction::MoveDown));
        }
        if target < current {
            return Some(AppAction::Navigation(NavigationAction::MoveUp));
        }
        return Some(AppAction::Branch(BranchAction::Checkout));
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

    if hit.rect == layout.commit_message {
        if state.staging_state.focus != StagingFocus::CommitMessage {
            return Some(AppAction::Staging(StagingAction::StartCommitMessage));
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::git::tests::test_utils::create_test_repo;
    use ratatui::layout::Rect;

    fn create_test_state() -> AppState {
        let (temp_dir, _repo) = create_test_repo();
        let git_repo = GitRepo::open(temp_dir.path().to_string_lossy().as_ref()).unwrap();
        let mut state =
            AppState::new(git_repo, temp_dir.path().to_string_lossy().to_string()).unwrap();
        state.screen_area = Rect::new(0, 0, 120, 40);
        state
    }

    #[test]
    fn test_search_mode_arrow_down_moves_to_next_result() {
        let mut state = create_test_state();
        state.search_state.is_active = true;

        let action = map_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &state);

        assert_eq!(action, Some(AppAction::Search(SearchAction::NextResult)));
    }

    #[test]
    fn test_search_mode_ctrl_n_moves_to_next_result() {
        let mut state = create_test_state();
        state.search_state.is_active = true;

        let action = map_key(
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
            &state,
        );

        assert_eq!(action, Some(AppAction::Search(SearchAction::NextResult)));
    }

    #[test]
    fn test_filter_popup_arrow_down_moves_to_next_field() {
        let mut state = create_test_state();
        state.filter_popup.is_open = true;

        let action = map_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &state);

        assert_eq!(action, Some(AppAction::Filter(FilterAction::NextField)));
    }

    #[test]
    fn test_ctrl_p_triggers_force_push() {
        let state = create_test_state();

        let action = map_key(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            &state,
        );

        assert_eq!(action, Some(AppAction::Git(GitAction::ForcePush)));
    }

    #[test]
    fn test_bottom_left_space_opens_diff_panel() {
        let mut state = create_test_state();
        state.focus = FocusPanel::BottomLeft;

        let action = map_key(
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
            &state,
        );

        assert_eq!(action, Some(AppAction::Select));
    }

    #[test]
    fn test_bottom_left_enter_opens_fullscreen_diff() {
        let mut state = create_test_state();
        state.focus = FocusPanel::BottomLeft;

        let action = map_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &state);

        assert_eq!(action, Some(AppAction::ToggleDiffFullscreen));
    }

    #[test]
    fn test_staging_space_opens_diff_panel() {
        let mut state = create_test_state();
        state.view_mode = ViewMode::Staging;
        state.staging_state.focus = StagingFocus::Staged;

        let action = map_key(
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
            &state,
        );

        assert_eq!(action, Some(AppAction::Staging(StagingAction::FocusDiff)));
    }

    #[test]
    fn test_mouse_click_nav_bar_switches_view() {
        let state = create_test_state();

        let action = map_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(event::MouseButton::Left),
                column: 20,
                row: 1,
                modifiers: KeyModifiers::NONE,
            },
            &state,
        );

        assert_eq!(action, Some(AppAction::SwitchView(ViewMode::Staging)));
    }

    #[test]
    fn test_mouse_scroll_graph_moves_down() {
        let state = create_test_state();

        let action = map_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &state,
        );

        assert_eq!(
            action,
            Some(AppAction::Navigation(NavigationAction::MoveDown))
        );
    }
}
