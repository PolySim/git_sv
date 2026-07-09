//! Actions sur les branches, worktrees et stashes.

#[derive(Debug, Clone, PartialEq)]
pub enum BranchAction {
    /// Ouvrir directement le selecteur de worktrees
    OpenWorktrees,
    /// Checkout une branche
    Checkout,
    /// Créer une branche
    Create,
    /// Supprimer une branche
    Delete,
    /// Renommer une branche
    Rename,
    /// Afficher/masquer les branches distantes
    ToggleRemote,
    /// Merger une branche
    Merge,
    /// Créer un stash
    StashSave,
    /// Appliquer un stash
    StashApply,
    /// Pop un stash
    StashPop,
    /// Supprimer un stash
    StashDrop,
    /// Fichier suivant dans le stash
    StashFileNext,
    /// Fichier précédent dans le stash
    StashFilePrev,
    /// Créer un worktree
    WorktreeCreate,
    /// Supprimer un worktree
    WorktreeRemove,
    /// Ouvrir le worktree selectionne
    WorktreeSwitch,
    /// Basculer vers la section suivante
    NextSection,
    /// Basculer vers la section précédente
    PrevSection,
    /// Confirmer l'input
    ConfirmInput,
    /// Annuler l'input
    CancelInput,
    /// Sélectionner une branche locale.
    SelectLocalBranch(usize),
    /// Sélectionner une branche distante.
    SelectRemoteBranch(usize),
    /// Sélectionner un worktree.
    SelectWorktree(usize),
    /// Sélectionner un stash.
    SelectStash(usize),
    /// Donner le focus à la liste.
    FocusList,
    /// Donner le focus au détail.
    FocusDetail,
}
