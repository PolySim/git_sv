//! Actions de l'application organisées par domaine.

#![allow(dead_code)]

mod branch;
mod conflict;
mod edit;
mod filter;
mod git;
mod navigation;
mod search;
mod staging;

pub use branch::BranchAction;
pub use conflict::ConflictAction;
pub use edit::EditAction;
pub use filter::FilterAction;
pub use git::GitAction;
pub use navigation::NavigationAction;
pub use search::SearchAction;
pub use staging::StagingAction;

use super::view::ViewMode;

/// Action principale de l'application.
///
/// Délègue vers des sous-enums spécialisés pour une meilleure organisation.
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    /// Quitter l'application
    Quit,

    /// Rafraîchir les données
    Refresh,

    /// Actions de navigation (nouvelle structure)
    Navigation(NavigationAction),

    /// Actions git (push, pull, fetch, etc.) (nouvelle structure)
    Git(GitAction),

    /// Actions de staging/commit (nouvelle structure)
    Staging(StagingAction),

    /// Actions sur les branches (nouvelle structure)
    Branch(BranchAction),

    /// Actions de résolution de conflits (nouvelle structure)
    Conflict(ConflictAction),

    /// Actions de recherche (nouvelle structure)
    Search(SearchAction),

    /// Actions d'édition de texte (nouvelle structure)
    Edit(EditAction),

    /// Actions de filtrage du graph (nouvelle structure)
    Filter(FilterAction),

    /// Changer de mode de vue
    SwitchView(ViewMode),

    /// Afficher/masquer l'aide
    ToggleHelp,

    /// Copier dans le presse-papier (nouvelle structure)
    CopyToClipboard,
    /// Copier le contenu du panneau (legacy - utiliser CopyToClipboard)
    CopyPanelContent,

    /// Sélectionner l'élément courant (Enter général)
    Select,

    /// Basculer le mode du panneau bas-gauche
    SwitchBottomMode,

    /// Fermer le panneau de branches
    CloseBranchPanel,

    /// Confirmer une action destructive
    ConfirmAction,

    /// Annuler une action destructive
    CancelAction,

    /// Navigation dans le merge picker
    MergePickerUp,
    MergePickerDown,
    MergePickerConfirm,
    MergePickerCancel,

    /// Navigation dans le reset picker
    ResetPickerSelectSoft,
    ResetPickerSelectHard,
    ResetPickerConfirm,
    ResetPickerCancel,

    /// Diff: Basculer entre mode unifié et side-by-side.
    ToggleDiffViewMode,

    /// Diff: Basculer le mode plein écran du diff.
    ToggleDiffFullscreen,

    /// Aucune action (événement ignoré)
    None,
}
