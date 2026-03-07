//! État de la vue branches/worktrees/stashes.

#![allow(dead_code)]

use crate::git::branch::BranchInfo;
use crate::git::stash::StashEntry;
use crate::git::worktree::WorktreeInfo;
use crate::state::selection::ListSelection;

/// Section active dans la vue branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BranchesSection {
    #[default]
    Branches,
    Worktrees,
    Stashes,
}

/// Panneau focalisé dans la vue branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BranchesFocus {
    #[default]
    List,
    Detail,
    Input,
}

/// Action d'input en cours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    CreateBranch,
    CreateWorktree,
    RenameBranch,
    SaveStash,
}

/// Représentation explicite de la branche sélectionnée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedBranch {
    /// Branche locale sélectionnée (index dans local_branches)
    Local(usize),
    /// Branche distante sélectionnée (index dans remote_branches)
    Remote(usize),
}

impl SelectedBranch {
    /// Retourne l'index dans la liste correspondante.
    pub fn index(&self) -> usize {
        match self {
            SelectedBranch::Local(idx) => *idx,
            SelectedBranch::Remote(idx) => *idx,
        }
    }

    /// Vérifie si c'est une branche locale.
    pub fn is_local(&self) -> bool {
        matches!(self, SelectedBranch::Local(_))
    }

    /// Vérifie si c'est une branche distante.
    pub fn is_remote(&self) -> bool {
        matches!(self, SelectedBranch::Remote(_))
    }
}

/// État de la vue branches/worktree/stash.
#[derive(Debug, Clone, Default)]
pub struct BranchesViewState {
    pub section: BranchesSection,
    pub focus: BranchesFocus,
    pub local_branches: ListSelection<BranchInfo>,
    pub remote_branches: ListSelection<BranchInfo>,
    /// Afficher les branches distantes dans la liste.
    pub show_remote: bool,
    /// Branche actuellement sélectionnée (explicite: locale ou distante).
    pub selected_branch: Option<SelectedBranch>,
    pub worktrees: ListSelection<WorktreeInfo>,
    pub stashes: ListSelection<StashEntry>,
    pub stash_file_selected: usize,
    pub stash_file_diff: Option<Vec<String>>,
    pub stash_diff_scroll: usize,
    pub input_text: String,
    pub input_cursor: usize,
    pub input_action: Option<InputAction>,
}

impl BranchesViewState {
    /// Crée un nouvel état branches.
    pub fn new() -> Self {
        let mut state = Self::default();
        // Par défaut, sélectionner la première branche locale si disponible
        state.selected_branch = Some(SelectedBranch::Local(0));
        state
    }

    /// Branche actuellement sélectionnée (retourne la référence et le type).
    pub fn selected_branch_info(&self) -> Option<(&BranchInfo, SelectedBranch)> {
        self.selected_branch.and_then(|selected| match selected {
            SelectedBranch::Local(idx) => {
                self.local_branches.get(idx).map(|b| (b, selected))
            }
            SelectedBranch::Remote(idx) => {
                self.remote_branches.get(idx).map(|b| (b, selected))
            }
        })
    }

    /// Retourne uniquement la branche sélectionnée (pour compatibilité ascendante).
    pub fn selected_branch(&self) -> Option<&BranchInfo> {
        self.selected_branch_info().map(|(branch, _)| branch)
    }

    /// Retourne true si une branche distante est sélectionnée.
    pub fn is_remote_selected(&self) -> bool {
        self.selected_branch
            .map(|s| s.is_remote())
            .unwrap_or(false)
    }

    /// Déplace la sélection vers le haut.
    pub fn select_prev(&mut self) {
        let local_count = self.local_branches.len();
        let remote_count = self.remote_branches.len();
        let show_remote = self.show_remote;

        self.selected_branch = match self.selected_branch {
            None => {
                // Par défaut, sélectionner la dernière branche disponible
                if show_remote && remote_count > 0 {
                    Some(SelectedBranch::Remote(remote_count - 1))
                } else if local_count > 0 {
                    Some(SelectedBranch::Local(local_count - 1))
                } else {
                    None
                }
            }
            Some(SelectedBranch::Local(0)) => {
                // Début des locales, ne rien faire ou aller aux remotes si visibles
                if show_remote && remote_count > 0 {
                    Some(SelectedBranch::Remote(remote_count - 1))
                } else {
                    Some(SelectedBranch::Local(0))
                }
            }
            Some(SelectedBranch::Local(idx)) => {
                Some(SelectedBranch::Local(idx.saturating_sub(1)))
            }
            Some(SelectedBranch::Remote(0)) => {
                // Début des remotes, remonter aux locales
                if local_count > 0 {
                    Some(SelectedBranch::Local(local_count - 1))
                } else if remote_count > 0 {
                    Some(SelectedBranch::Remote(0))
                } else {
                    None
                }
            }
            Some(SelectedBranch::Remote(idx)) => {
                Some(SelectedBranch::Remote(idx.saturating_sub(1)))
            }
        };
    }

    /// Déplace la sélection vers le bas.
    pub fn select_next(&mut self) {
        let local_count = self.local_branches.len();
        let remote_count = self.remote_branches.len();
        let show_remote = self.show_remote;

        self.selected_branch = match self.selected_branch {
            None => {
                // Par défaut, sélectionner la première branche locale
                if local_count > 0 {
                    Some(SelectedBranch::Local(0))
                } else if show_remote && remote_count > 0 {
                    Some(SelectedBranch::Remote(0))
                } else {
                    None
                }
            }
            Some(SelectedBranch::Local(idx)) => {
                if idx + 1 < local_count {
                    Some(SelectedBranch::Local(idx + 1))
                } else if show_remote && remote_count > 0 {
                    // Passer aux remotes
                    Some(SelectedBranch::Remote(0))
                } else {
                    Some(SelectedBranch::Local(idx))
                }
            }
            Some(SelectedBranch::Remote(idx)) => {
                if idx + 1 < remote_count {
                    Some(SelectedBranch::Remote(idx + 1))
                } else {
                    // Fin des remotes, retourner au début ou rester
                    if local_count > 0 {
                        Some(SelectedBranch::Local(0))
                    } else {
                        Some(SelectedBranch::Remote(idx))
                    }
                }
            }
        };
    }

    /// Bascule entre l'affichage des branches distantes.
    pub fn toggle_remote(&mut self) {
        self.show_remote = !self.show_remote;
        // Si on cache les remotes et qu'une remote est sélectionnée,
        // revenir sur une branche locale
        if !self.show_remote {
            if let Some(SelectedBranch::Remote(_)) = self.selected_branch {
                if self.local_branches.len() > 0 {
                    self.selected_branch = Some(SelectedBranch::Local(0));
                } else {
                    self.selected_branch = None;
                }
            }
        }
    }

    // ═══════════════════════════════════════════════════
    // Compatibilité ascendante - accesseurs legacy
    // ═══════════════════════════════════════════════════

    /// Index de la branche sélectionnée pour le rendu (compatibilité).
    /// Retourne l'index dans la liste combinée (avec headers).
    pub fn branch_selected(&self) -> usize {
        match self.selected_branch {
            Some(SelectedBranch::Local(idx)) => idx,
            Some(SelectedBranch::Remote(idx)) => {
                // Pour les remotes, l'index dans remote_branches
                // Le calcul visuel est fait dans le rendu
                idx
            }
            None => 0,
        }
    }

    /// Définit l'index de la branche sélectionnée (compatibilité - assume local).
    pub fn set_branch_selected(&mut self, index: usize) {
        self.selected_branch = Some(SelectedBranch::Local(index));
    }

    /// Index du stash sélectionné (compatibilité).
    pub fn stash_selected(&self) -> usize {
        self.stashes.selected_index()
    }

    /// Définit l'index du stash sélectionné (compatibilité).
    pub fn set_stash_selected(&mut self, index: usize) {
        self.stashes.select(index);
    }

    /// Index du worktree sélectionné (compatibilité).
    pub fn worktree_selected(&self) -> usize {
        self.worktrees.selected_index()
    }

    /// Définit l'index du worktree sélectionné (compatibilité).
    pub fn set_worktree_selected(&mut self, index: usize) {
        self.worktrees.select(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::branch::BranchInfo;

    fn create_test_branch(name: &str, is_head: bool) -> BranchInfo {
        BranchInfo {
            name: name.to_string(),
            is_head,
            is_remote: false,
            last_commit_message: None,
            last_commit_date: None,
            ahead: None,
            behind: None,
        }
    }

    fn create_test_remote_branch(name: &str) -> BranchInfo {
        BranchInfo {
            name: name.to_string(),
            is_head: false,
            is_remote: true,
            last_commit_message: None,
            last_commit_date: None,
            ahead: None,
            behind: None,
        }
    }

    #[test]
    fn test_new_state_selects_first_local() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
            create_test_branch("feature", false),
        ]);

        assert_eq!(state.selected_branch, Some(SelectedBranch::Local(0)));
    }

    #[test]
    fn test_select_next_local() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
            create_test_branch("feature", false),
        ]);
        state.selected_branch = Some(SelectedBranch::Local(0));

        state.select_next();

        assert_eq!(state.selected_branch, Some(SelectedBranch::Local(1)));
    }

    #[test]
    fn test_select_next_to_remote() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
        ]);
        state.remote_branches.set_items(vec![
            create_test_remote_branch("origin/main"),
            create_test_remote_branch("origin/feature"),
        ]);
        state.show_remote = true;
        state.selected_branch = Some(SelectedBranch::Local(0));

        state.select_next();

        assert_eq!(state.selected_branch, Some(SelectedBranch::Remote(0)));
    }

    #[test]
    fn test_select_next_wraps_to_beginning() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
        ]);
        state.remote_branches.set_items(vec![
            create_test_remote_branch("origin/main"),
        ]);
        state.show_remote = true;
        state.selected_branch = Some(SelectedBranch::Remote(0));

        state.select_next();

        assert_eq!(state.selected_branch, Some(SelectedBranch::Local(0)));
    }

    #[test]
    fn test_select_prev_local() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
            create_test_branch("feature", false),
        ]);
        state.selected_branch = Some(SelectedBranch::Local(1));

        state.select_prev();

        assert_eq!(state.selected_branch, Some(SelectedBranch::Local(0)));
    }

    #[test]
    fn test_select_prev_to_remote() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
        ]);
        state.remote_branches.set_items(vec![
            create_test_remote_branch("origin/main"),
        ]);
        state.show_remote = true;
        state.selected_branch = Some(SelectedBranch::Local(0));

        state.select_prev();

        assert_eq!(state.selected_branch, Some(SelectedBranch::Remote(0)));
    }

    #[test]
    fn test_toggle_remote_hides_remote_selection() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
            create_test_branch("feature", false),
        ]);
        state.remote_branches.set_items(vec![
            create_test_remote_branch("origin/main"),
        ]);
        state.show_remote = true;
        state.selected_branch = Some(SelectedBranch::Remote(0));

        state.toggle_remote();

        assert!(!state.show_remote);
        assert_eq!(state.selected_branch, Some(SelectedBranch::Local(0)));
    }

    #[test]
    fn test_toggle_remote_when_no_local() {
        let mut state = BranchesViewState::new();
        state.remote_branches.set_items(vec![
            create_test_remote_branch("origin/main"),
        ]);
        state.show_remote = true;
        state.selected_branch = Some(SelectedBranch::Remote(0));

        state.toggle_remote();

        assert!(!state.show_remote);
        assert_eq!(state.selected_branch, None);
    }

    #[test]
    fn test_is_remote_selected() {
        let mut state = BranchesViewState::new();
        state.selected_branch = Some(SelectedBranch::Remote(0));

        assert!(state.is_remote_selected());

        state.selected_branch = Some(SelectedBranch::Local(0));

        assert!(!state.is_remote_selected());
    }

    #[test]
    fn test_selected_branch_info() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
        ]);
        state.selected_branch = Some(SelectedBranch::Local(0));

        let (branch, selected) = state.selected_branch_info().unwrap();

        assert_eq!(branch.name, "main");
        assert!(selected.is_local());
    }

    #[test]
    fn test_select_prev_wraps_from_first_local() {
        let mut state = BranchesViewState::new();
        state.local_branches.set_items(vec![
            create_test_branch("main", true),
            create_test_branch("feature", false),
        ]);
        state.remote_branches.set_items(vec![
            create_test_remote_branch("origin/main"),
        ]);
        state.show_remote = true;
        state.selected_branch = Some(SelectedBranch::Local(0));

        state.select_prev();

        assert_eq!(state.selected_branch, Some(SelectedBranch::Remote(0)));
    }
}
