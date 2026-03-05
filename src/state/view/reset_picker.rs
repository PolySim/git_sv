//! État du sélecteur de type de reset.

#![allow(dead_code)]

use git2::Oid;

/// État du sélecteur de type de reset.
#[derive(Debug, Clone)]
pub struct ResetPickerState {
    /// OID du commit cible.
    pub target_oid: Oid,
    /// Hash court du commit pour l'affichage.
    pub short_hash: String,
    /// Message du commit.
    pub commit_message: String,
    /// Actif ou non.
    pub is_active: bool,
    /// Option sélectionnée : 0 = Soft, 1 = Hard.
    pub selected_index: usize,
}

impl ResetPickerState {
    /// Crée un nouveau reset picker.
    pub fn new(target_oid: Oid, short_hash: String, commit_message: String) -> Self {
        Self {
            target_oid,
            short_hash,
            commit_message,
            is_active: true,
            selected_index: 0,
        }
    }

    /// Bascule entre les options (Soft/Hard).
    pub fn toggle(&mut self) {
        self.selected_index = if self.selected_index == 0 { 1 } else { 0 };
    }

    /// Retourne true si Soft est sélectionné.
    pub fn is_soft_selected(&self) -> bool {
        self.selected_index == 0
    }

    /// Retourne true si Hard est sélectionné.
    pub fn is_hard_selected(&self) -> bool {
        self.selected_index == 1
    }
}
