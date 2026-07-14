//! Actions de navigation dans les listes et panneaux.

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationAction {
    /// Monter d'un élément
    MoveUp,
    /// Descendre d'un élément
    MoveDown,
    /// Remonter d'une page
    PageUp,
    /// Descendre d'une page
    PageDown,
    /// Aller au premier élément
    GoTop,
    /// Aller au dernier élément
    GoBottom,
    /// Changer de panneau (Tab)
    SwitchPanel,
    /// Faire défiler le diff vers le haut (ligne par ligne)
    ScrollDiffUp,
    /// Faire défiler le diff vers le bas (ligne par ligne)
    ScrollDiffDown,
    /// Faire défiler le diff d'une page vers le haut
    ScrollDiffPageUp,
    /// Faire défiler le diff d'une page vers le bas
    ScrollDiffPageDown,
    /// Aller au début du diff
    ScrollDiffTop,
    /// Aller à la fin du diff
    ScrollDiffBottom,
    /// Faire défiler le diff vers la gauche (horizontal)
    ScrollDiffLeft,
    /// Faire défiler le diff vers la droite (horizontal)
    ScrollDiffRight,
    /// Aller au hunk suivant dans le diff.
    NextDiffHunk,
    /// Aller au hunk précédent dans le diff.
    PreviousDiffHunk,
    /// Faire défiler le diff de stash vers le haut
    ScrollStashDiffUp,
    /// Faire défiler le diff de stash vers le bas
    ScrollStashDiffDown,
    /// Naviguer vers le haut dans le panneau de fichiers
    FileUp,
    /// Naviguer vers le bas dans le panneau de fichiers
    FileDown,
    /// Retourner au focus Graph (depuis BottomLeft/Files)
    BackToGraph,
    /// Donner explicitement le focus au graphe.
    FocusGraph,
    /// Donner explicitement le focus au panneau bas-gauche.
    FocusBottomLeft,
    /// Donner explicitement le focus au panneau bas-droit.
    FocusBottomRight,
    /// Sélectionner un commit spécifique par son index (souris)
    SelectCommit(usize),
    /// Sélectionner un fichier spécifique par son index (souris)
    SelectFile(usize),
}
