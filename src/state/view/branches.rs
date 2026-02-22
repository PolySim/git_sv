//! État de la vue branches/worktrees/stashes.

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

/// État de la vue branches/worktree/stash.
#[derive(Debug, Clone, Default)]
pub struct BranchesViewState {
    pub section: BranchesSection,
    pub focus: BranchesFocus,
    pub local_branches: ListSelection<BranchInfo>,
    pub remote_branches: ListSelection<BranchInfo>,
    pub show_remote: bool,
    pub worktrees: ListSelection<WorktreeInfo>,
    pub stashes: ListSelection<StashEntry>,
    pub stash_file_selected: usize,
    pub stash_file_diff: Option<Vec<String>>,
    pub input_text: String,
    pub input_cursor: usize,
    pub input_action: Option<InputAction>,
}

impl BranchesViewState {
    /// Crée un nouvel état branches.
    pub fn new() -> Self {
        Self::default()
    }

    /// Branche actuellement sélectionnée (locale ou remote selon affichage).
    pub fn selected_branch(&self) -> Option<&BranchInfo> {
        if self.show_remote {
            self.remote_branches.selected_item()
        } else {
            self.local_branches.selected_item()
        }
    }

    // ═══════════════════════════════════════════════════
    // Compatibilité ascendante - accesseurs legacy
    // ═══════════════════════════════════════════════════

    /// Index de la branche sélectionnée (compatibilité - délègue vers ListSelection).
    pub fn branch_selected(&self) -> usize {
        if self.show_remote {
            self.remote_branches.selected_index()
        } else {
            self.local_branches.selected_index()
        }
    }

    /// Définit l'index de la branche sélectionnée (compatibilité).
    pub fn set_branch_selected(&mut self, index: usize) {
        if self.show_remote {
            self.remote_branches.select(index);
        } else {
            self.local_branches.select(index);
        }
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
