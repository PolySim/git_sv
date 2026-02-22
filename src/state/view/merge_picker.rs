//! État du sélecteur de branche pour le merge.

use crate::state::selection::ListSelection;

/// État du sélecteur de branche pour le merge.
#[derive(Debug, Clone, Default)]
pub struct MergePickerState {
    /// Liste des branches disponibles (hors branche courante).
    pub branches: ListSelection<String>,
    /// Actif ou non.
    pub is_active: bool,
}

impl MergePickerState {
    /// Crée un nouveau merge picker.
    pub fn new(branches: Vec<String>) -> Self {
        Self {
            branches: ListSelection::with_items(branches),
            is_active: true,
        }
    }

    /// Branche actuellement sélectionnée.
    pub fn selected_branch(&self) -> Option<&String> {
        self.branches.selected_item()
    }

    /// Index de la branche sélectionnée (compatibilité ascendante).
    pub fn selected(&self) -> usize {
        self.branches.selected_index()
    }

    /// Définit l'index de la branche sélectionnée (compatibilité).
    pub fn set_selected(&mut self, index: usize) {
        self.branches.select(index);
    }
}
