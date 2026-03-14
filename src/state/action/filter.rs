//! Actions de filtrage pour le graph de commits.

/// Actions de filtrage du graph.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterAction {
    /// Ouvrir le popup de filtre.
    Open,
    /// Fermer le popup de filtre.
    Close,
    /// Passer au champ suivant.
    NextField,
    /// Passer au champ précédent.
    PreviousField,
    /// Insérer un caractère dans le champ actuel.
    InsertChar(char),
    /// Supprimer un caractère dans le champ actuel.
    DeleteChar,
    /// Appliquer les filtres.
    Apply,
    /// Effacer tous les filtres.
    Clear,
}
