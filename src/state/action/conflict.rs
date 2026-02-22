//! Actions de résolution de conflits.

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictAction {
    /// Naviguer vers le fichier précédent
    PreviousFile,
    /// Naviguer vers le fichier suivant
    NextFile,
    /// Naviguer vers la section précédente
    PreviousSection,
    /// Naviguer vers la section suivante
    NextSection,
    /// Changer de panneau
    SwitchPanel,
    /// Accepter notre version (fichier entier)
    AcceptOursFile,
    /// Accepter leur version (fichier entier)
    AcceptTheirsFile,
    /// Accepter notre version (bloc)
    AcceptOursBlock,
    /// Accepter leur version (bloc)
    AcceptTheirsBlock,
    /// Accepter les deux versions
    AcceptBoth,
    /// Activer le mode édition
    StartEdit,
    /// Valider l'édition
    ConfirmEdit,
    /// Annuler l'édition
    CancelEdit,
    /// Marquer le fichier comme résolu
    MarkResolved,
    /// Finaliser le merge
    FinalizeMerge,
    /// Abandonner le merge
    AbortMerge,
    /// Définir le mode de résolution à Fichier
    SetModeFile,
    /// Définir le mode de résolution à Bloc
    SetModeBlock,
    /// Définir le mode de résolution à Ligne
    SetModeLine,
    /// En mode ligne : toggle l'inclusion de la ligne sélectionnée
    ToggleLine,
    /// En mode ligne : sélectionner la ligne suivante
    LineDown,
    /// En mode ligne : sélectionner la ligne précédente
    LineUp,
    /// Scroll vers le bas dans le panneau résultat
    ResultScrollDown,
    /// Scroll vers le haut dans le panneau résultat
    ResultScrollUp,
    /// Entrer en mode édition dans le panneau résultat
    StartEditing,
    /// Quitter le mode édition
    StopEditing,
    /// Insérer un caractère en mode édition
    EditInsertChar(char),
    /// Supprimer le caractère avant le curseur
    EditBackspace,
    /// Supprimer le caractère sous le curseur
    EditDelete,
    /// Déplacer le curseur en mode édition
    EditCursorUp,
    EditCursorDown,
    EditCursorLeft,
    EditCursorRight,
    /// Insérer une nouvelle ligne
    EditNewline,
    /// Quitter la vue conflits
    LeaveView,
    /// Valider la résolution (Enter contextuel)
    EnterResolve,
}
