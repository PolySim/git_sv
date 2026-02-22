//! Actions sur les branches, worktrees et stashes.

#[derive(Debug, Clone, PartialEq)]
pub enum BranchAction {
    /// Lister les branches
    List,
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
    /// Créer un worktree
    WorktreeCreate,
    /// Supprimer un worktree
    WorktreeRemove,
    /// Basculer vers la section suivante
    NextSection,
    /// Basculer vers la section précédente
    PrevSection,
    /// Confirmer l'input
    ConfirmInput,
    /// Annuler l'input
    CancelInput,
}
