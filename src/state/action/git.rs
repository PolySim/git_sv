//! Actions git (opérations remote, etc.)

#![allow(dead_code)]

/// Mode de reset pour l'action ResetToCommit.
#[derive(Debug, Clone, PartialEq)]
pub enum ResetMode {
    /// Soft reset : déplace HEAD, garde les modifications dans l'index (staged)
    Soft,
    /// Hard reset : déplace HEAD, réinitialise l'index ET le working directory
    Hard,
}

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
    /// Lister les branches
    BranchList,
    /// Ouvrir le dialogue de reset
    ResetPrompt,
    /// Annuler le merge en cours
    AbortMerge,
}
