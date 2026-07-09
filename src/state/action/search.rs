//! Actions de recherche.

use super::EditAction;

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    /// Ouvrir la recherche
    Open,
    /// Fermer la recherche
    Close,
    /// Insérer un caractère dans la recherche
    InsertChar(char),
    /// Supprimer le caractère avant le curseur
    DeleteChar,
    /// Editer le champ avec les raccourcis usuels.
    Edit(EditAction),
    /// Résultat suivant
    NextResult,
    /// Résultat précédent
    PreviousResult,
    /// Changer le type de recherche
    ChangeType,
    /// Exécuter la recherche
    Execute,
}
