//! Actions de recherche.

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    /// Ouvrir la recherche
    Open,
    /// Fermer la recherche
    Close,
    /// Résultat suivant
    NextResult,
    /// Résultat précédent
    PreviousResult,
    /// Changer le type de recherche
    ChangeType,
    /// Exécuter la recherche
    Execute,
}
