//! Actions d'édition de texte.

#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    /// Insérer un caractère
    InsertChar(char),
    /// Supprimer le caractère avant le curseur
    DeleteCharBefore,
    /// Supprimer le caractère après le curseur
    DeleteCharAfter,
    /// Déplacer le curseur à gauche
    CursorLeft,
    /// Déplacer le curseur à droite
    CursorRight,
    /// Aller au début de la ligne
    CursorHome,
    /// Aller à la fin de la ligne
    CursorEnd,
    /// Nouvelle ligne
    NewLine,
}
