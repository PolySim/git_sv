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
    /// Faire défiler le diff vers le haut
    ScrollDiffUp,
    /// Faire défiler le diff vers le bas
    ScrollDiffDown,
    /// Naviguer vers le haut dans le panneau de fichiers
    FileUp,
    /// Naviguer vers le bas dans le panneau de fichiers
    FileDown,
    /// Retourner au focus Graph (depuis BottomLeft/Files)
    BackToGraph,
}
