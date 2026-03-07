//! Actions de staging et commit.

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum StagingAction {
    /// Ajouter un fichier au staging
    StageFile,
    /// Retirer un fichier du staging
    UnstageFile,
    /// Ajouter tous les fichiers
    StageAll,
    /// Retirer tous les fichiers
    UnstageAll,
    /// Commencer l'édition du message de commit
    StartCommitMessage,
    /// Valider le commit
    ConfirmCommit,
    /// Annuler le commit
    CancelCommit,
    /// Discard les modifications d'un fichier
    DiscardFile,
    /// Discard toutes les modifications
    DiscardAll,
    /// Changer le focus dans la vue staging
    SwitchFocus,
    /// Ouvrir le panneau diff depuis la liste active
    FocusDiff,
    /// Stash le fichier sélectionné
    StashSelectedFile,
    /// Stash tous les fichiers non stagés
    StashUnstagedFiles,
    /// Donner le focus à la liste unstaged.
    FocusUnstaged,
    /// Donner le focus à la liste staged.
    FocusStaged,
    /// Sélectionner un fichier unstaged par index.
    SelectUnstaged(usize),
    /// Sélectionner un fichier staged par index.
    SelectStaged(usize),
}
