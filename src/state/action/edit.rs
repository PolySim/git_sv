//! Actions d'édition de texte.

#![allow(dead_code)]

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
    /// Déplacer le curseur d'un mot vers la gauche
    CursorWordLeft,
    /// Déplacer le curseur d'un mot vers la droite
    CursorWordRight,
    /// Etendre la selection d'un caractère vers la gauche
    SelectLeft,
    /// Etendre la selection d'un caractère vers la droite
    SelectRight,
    /// Etendre la selection d'un mot vers la gauche
    SelectWordLeft,
    /// Etendre la selection d'un mot vers la droite
    SelectWordRight,
    /// Aller au début de la ligne
    CursorHome,
    /// Aller à la fin de la ligne
    CursorEnd,
    /// Etendre la selection jusqu'au debut
    SelectHome,
    /// Etendre la selection jusqu'a la fin
    SelectEnd,
    /// Supprimer le mot avant le curseur
    DeleteWordBefore,
    /// Supprimer du curseur jusqu'au debut
    DeleteToStart,
    /// Supprimer du curseur jusqu'a la fin
    DeleteToEnd,
    /// Selectionner tout le contenu
    SelectAll,
    /// Annuler la derniere modification
    Undo,
    /// Retablir la derniere modification annulee
    Redo,
    /// Nouvelle ligne
    NewLine,
}
