use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::time::Duration;

use crate::state::{
    AppAction, AppState, BranchesFocus, BranchesSection, ConflictPanelFocus, FocusPanel,
    StagingFocus, ViewMode,
};
use crate::state::action::SearchAction;

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

    // Si une confirmation est en attente, gérer y/n/ESC
    if state.pending_confirmation.is_some() {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(AppAction::ConfirmAction),
            KeyCode::Char('n') | KeyCode::Char('N') => Some(AppAction::CancelAction),
            KeyCode::Esc => Some(AppAction::CancelAction),
            _ => None,
        };
    }

    // Si la recherche est active, gérer les inputs de recherche
    if state.search_state.is_active {
        return match key.code {
            KeyCode::Esc => Some(AppAction::Search(SearchAction::Close)),
            KeyCode::Enter => Some(AppAction::Search(SearchAction::Execute)),
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
            KeyCode::Esc => Some(AppAction::CloseFilter),
            KeyCode::Enter => Some(AppAction::ApplyFilter),
            KeyCode::Tab | KeyCode::Down => Some(AppAction::FilterNextField),
            KeyCode::BackTab | KeyCode::Up => Some(AppAction::FilterPrevField),
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(AppAction::ClearFilter)
            }
            KeyCode::Char(c) => Some(AppAction::FilterInsertChar(c)),
            KeyCode::Backspace => Some(AppAction::FilterDeleteChar),
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
        KeyCode::Char('1') => return Some(AppAction::SwitchToGraph),
        KeyCode::Char('2') => return Some(AppAction::SwitchToStaging),
        KeyCode::Char('3') => return Some(AppAction::SwitchToBranches),
        KeyCode::Char('4') => {
            if state.conflicts_state.is_some() {
                return Some(AppAction::SwitchToConflicts);
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
                KeyCode::Char('v') => return Some(AppAction::ToggleDiffViewMode),
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
        KeyCode::Char('P') => Some(AppAction::GitPush),
        KeyCode::Char('p') => Some(AppAction::GitPull),
        KeyCode::Char('f') => Some(AppAction::GitFetch),

        // Recherche
        KeyCode::Char('/') => Some(AppAction::OpenSearch),
        KeyCode::Char('n') => Some(AppAction::NextSearchResult),
        KeyCode::Char('N') => Some(AppAction::PrevSearchResult),

        // Filtre
        KeyCode::Char('F') => Some(AppAction::OpenFilter),

        // Vue blame
        KeyCode::Char('B') => Some(AppAction::OpenBlame),

        // Cherry-pick
        KeyCode::Char('x') => Some(AppAction::CherryPick),

        // Aide
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),

        // Rafraîchir
        KeyCode::Char('r') => Some(AppAction::Refresh),

        // Copier le contenu du panneau actif dans le clipboard
        KeyCode::Char('y') => Some(AppAction::CopyPanelContent),

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
        KeyCode::Char('y') => return Some(AppAction::CopyPanelContent),
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp),
        KeyCode::Char('P') => return Some(AppAction::GitPush),
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
            KeyCode::Char('m') => Some(AppAction::MergePrompt),
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
            KeyCode::Char('l') | KeyCode::Right => Some(AppAction::FileDown),
            KeyCode::Char('h') | KeyCode::Left => Some(AppAction::FileUp),
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
    // Vérifier d'abord si on est en mode saisie de commit
    if state.staging_state.focus == StagingFocus::CommitMessage {
        return match key.code {
            KeyCode::Enter => Some(AppAction::ConfirmCommit),
            KeyCode::Esc => Some(AppAction::CancelCommitMessage),
            KeyCode::Char(c) => Some(AppAction::InsertChar(c)),
            KeyCode::Backspace => Some(AppAction::DeleteChar),
            KeyCode::Left => Some(AppAction::MoveCursorLeft),
            KeyCode::Right => Some(AppAction::MoveCursorRight),
            _ => None,
        };
    }

    // Touches globales de la vue staging
    match key.code {
        KeyCode::Char('q') => return Some(AppAction::Quit),
        KeyCode::Char('r') => return Some(AppAction::Refresh),
        KeyCode::Char('y') => return Some(AppAction::CopyPanelContent),
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp),
        KeyCode::Char('P') => return Some(AppAction::GitPush),
        _ => {}
    }

    // Navigation selon le focus dans la vue staging
    match state.staging_state.focus {
        StagingFocus::Unstaged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('s') | KeyCode::Enter => Some(AppAction::StageFile),
            KeyCode::Char('S') => Some(AppAction::StashSelectedFile),
            KeyCode::Char('a') => Some(AppAction::StageAll),
            KeyCode::Char('d') => Some(AppAction::DiscardFile),
            KeyCode::Char('D') => Some(AppAction::DiscardAll),
            KeyCode::Tab => Some(AppAction::SwitchStagingFocus),
            KeyCode::Char('c') => Some(AppAction::StartCommitMessage),
            _ if key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('S') =>
            {
                Some(AppAction::StashUnstagedFiles)
            }
            _ => None,
        },
        StagingFocus::Staged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('u') | KeyCode::Enter => Some(AppAction::UnstageFile),
            KeyCode::Char('U') => Some(AppAction::UnstageAll),
            KeyCode::Tab => Some(AppAction::SwitchStagingFocus),
            KeyCode::Char('c') => Some(AppAction::StartCommitMessage),
            KeyCode::Char('A') => Some(AppAction::AmendCommit),
            _ => None,
        },
        StagingFocus::Diff => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::DiffScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::DiffScrollUp),
            KeyCode::Tab | KeyCode::Esc => Some(AppAction::SwitchStagingFocus),
            KeyCode::Char('c') => Some(AppAction::StartCommitMessage),
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
        KeyCode::Char('q') | KeyCode::Esc => Some(AppAction::CloseBlame),
        KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
        KeyCode::Char('g') | KeyCode::Home => Some(AppAction::GoTop),
        KeyCode::Char('G') | KeyCode::End => Some(AppAction::GoBottom),
        KeyCode::PageUp => Some(AppAction::PageUp),
        KeyCode::PageDown => Some(AppAction::PageDown),
        KeyCode::Enter => Some(AppAction::JumpToBlameCommit),
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
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(AppAction::ConfirmAction),
            KeyCode::Char('n') | KeyCode::Char('N') => Some(AppAction::CancelAction),
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
            KeyCode::Esc => Some(AppAction::ConflictStopEditing),
            KeyCode::Char(c) => Some(AppAction::ConflictEditInsertChar(c)),
            KeyCode::Backspace => Some(AppAction::ConflictEditBackspace),
            KeyCode::Delete => Some(AppAction::ConflictEditDelete),
            KeyCode::Enter => Some(AppAction::ConflictEditNewline),
            KeyCode::Up => Some(AppAction::ConflictEditCursorUp),
            KeyCode::Down => Some(AppAction::ConflictEditCursorDown),
            KeyCode::Left => Some(AppAction::ConflictEditCursorLeft),
            KeyCode::Right => Some(AppAction::ConflictEditCursorRight),
            _ => None,
        };
    }

    match key.code {
        // Tab et Shift+Tab : basculer entre les panneaux
        KeyCode::Tab => Some(AppAction::ConflictSwitchPanelForward),
        KeyCode::BackTab => Some(AppAction::ConflictSwitchPanelReverse),

        // Navigation flèches/j/k : dépend du panneau actif et du mode de résolution
        KeyCode::Char('j') | KeyCode::Down => match panel_focus {
            Some(ConflictPanelFocus::FileList) => Some(AppAction::ConflictNextFile),
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    // En mode Fichier, naviguer entre les fichiers (pas entre sections)
                    ConflictResolutionMode::File => Some(AppAction::ConflictNextFile),
                    ConflictResolutionMode::Line => Some(AppAction::ConflictLineDown),
                    ConflictResolutionMode::Block => Some(AppAction::ConflictNextSection),
                }
            }
            Some(ConflictPanelFocus::ResultPanel) => Some(AppAction::ConflictResultScrollDown),
            _ => None,
        },
        KeyCode::Char('k') | KeyCode::Up => match panel_focus {
            Some(ConflictPanelFocus::FileList) => Some(AppAction::ConflictPrevFile),
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                match resolution_mode {
                    // En mode Fichier, naviguer entre les fichiers (pas entre sections)
                    ConflictResolutionMode::File => Some(AppAction::ConflictPrevFile),
                    ConflictResolutionMode::Line => Some(AppAction::ConflictLineUp),
                    ConflictResolutionMode::Block => Some(AppAction::ConflictPrevSection),
                }
            }
            Some(ConflictPanelFocus::ResultPanel) => Some(AppAction::ConflictResultScrollUp),
            _ => None,
        },

        // Résolution rapide depuis le panneau FileList
        KeyCode::Char('o') | KeyCode::Left => match panel_focus {
            Some(ConflictPanelFocus::FileList) => Some(AppAction::ConflictFileChooseOurs),
            _ => None,
        },
        KeyCode::Char('t') | KeyCode::Right => match panel_focus {
            Some(ConflictPanelFocus::FileList) => Some(AppAction::ConflictFileChooseTheirs),
            _ => None,
        },

        // Résolution "Both" uniquement en mode Bloc (depuis les panneaux Ours/Theirs)
        KeyCode::Char('b') => {
            if matches!(
                panel_focus,
                Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel)
            ) && resolution_mode == ConflictResolutionMode::Block
            {
                Some(AppAction::ConflictChooseBoth)
            } else {
                None
            }
        }

        // Mode édition (panneau résultat uniquement)
        KeyCode::Char('i') | KeyCode::Char('e') => {
            if panel_focus == Some(ConflictPanelFocus::ResultPanel) {
                Some(AppAction::ConflictStartEditing)
            } else {
                None
            }
        }

        // Changement de mode de résolution (mapping direct)
        KeyCode::Char('F') => Some(AppAction::ConflictSetModeFile),
        KeyCode::Char('B') => Some(AppAction::ConflictSetModeBlock),
        KeyCode::Char('L') => Some(AppAction::ConflictSetModeLine),

        // Enter: validation contextuelle selon le panneau et le mode
        KeyCode::Enter => match panel_focus {
            Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
                // En mode Ligne, Enter toggle la ligne courante
                // En mode Fichier/Bloc, Enter résout selon le panneau actif
                Some(AppAction::ConflictEnterResolve)
            }
            _ => None,
        },
        KeyCode::Char('V') => Some(AppAction::ConflictValidateMerge),
        KeyCode::Char('q') | KeyCode::Esc => {
            if is_editing {
                Some(AppAction::ConflictStopEditing)
            } else {
                Some(AppAction::ConflictLeaveView)
            }
        }
        KeyCode::Char('A') => Some(AppAction::ConflictAbort),

        // Vues
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),

        // Navigation entre vues
        KeyCode::Char('1') => return Some(AppAction::SwitchToGraph),
        KeyCode::Char('2') => return Some(AppAction::SwitchToStaging),
        KeyCode::Char('3') => return Some(AppAction::SwitchToBranches),
        _ => None,
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
                ViewMode::Blame => Some(AppAction::MoveUp),
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
                ViewMode::Blame => Some(AppAction::MoveDown),
                _ => None,
            }
        }
        _ => None,
    }
}
