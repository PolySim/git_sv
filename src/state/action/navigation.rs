//! Actions de navigation dans les listes et panneaux.

#![allow(dead_code)]

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
    /// Naviguer vers le haut dans le panneau de fichiers
    FileUp,
    /// Naviguer vers le bas dans le panneau de fichiers
    FileDown,
    /// Retourner au focus Graph (depuis BottomLeft/Files)
    BackToGraph,
}
