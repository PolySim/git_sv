//! Actions git (opérations remote, etc.)

#[derive(Debug, Clone, PartialEq)]
pub enum GitAction {
    /// Push vers le remote
    Push,
    /// Force push vers le remote
    ForcePush,
    /// Pull depuis le remote
    Pull,
    /// Fetch depuis le remote
    Fetch,
    /// Cherry-pick un commit
    CherryPick,
    /// Amender le dernier commit
    AmendCommit,
    /// Ouvrir le blame d'un fichier
    OpenBlame,
    /// Fermer le blame
    CloseBlame,
    /// Aller au commit du blame
    JumpToBlameCommit,
    /// Ouvrir le dialogue de commit
    CommitPrompt,
    /// Ouvrir le dialogue de stash
    StashPrompt,
    /// Ouvrir le dialogue de merge
    MergePrompt,
    /// Ouvrir le dialogue de rebase
    RebasePrompt,
    /// Lancer un rebase interactif depuis le commit sélectionné.
    InteractiveRebase,
    /// Annuler la dernière transition de HEAD à partir du reflog.
    UndoLastOperation,
    /// Créer un tag sur le commit sélectionné.
    CreateTag,
    /// Supprimer un tag présent sur le commit sélectionné.
    DeleteTag,
    /// Ouvrir le selecteur de branche pour une comparaison d'historique
    ComparePrompt,
    /// Quitter la comparaison d'historique active
    ClearComparison,
    /// Comparer le commit sélectionné à HEAD.
    CompareSelectedWithHead,
    /// Démarrer un bisect avec le commit sélectionné comme référence bonne.
    BisectStart,
    /// Marquer le commit courant du bisect comme bon.
    BisectGood,
    /// Marquer le commit courant du bisect comme mauvais.
    BisectBad,
    /// Terminer le bisect et restaurer la branche initiale.
    BisectReset,
    /// Ouvrir le dialogue de reset
    ResetPrompt,
    /// Annuler le merge en cours
    AbortMerge,
}
