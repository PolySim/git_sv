//! État du sélecteur de type de reset.

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
    /// Option sélectionnée : 0 = Soft, 1 = Mixed, 2 = Hard.
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

    /// Retourne true si Soft est sélectionné.
    pub fn is_soft_selected(&self) -> bool {
        self.selected_index == 0
    }

    /// Retourne true si Mixed est sélectionné.
    pub fn is_mixed_selected(&self) -> bool {
        self.selected_index == 1
    }

    /// Retourne true si Hard est sélectionné.
    pub fn is_hard_selected(&self) -> bool {
        self.selected_index == 2
    }

    /// Sélectionne l'option précédente.
    pub fn select_previous(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// Sélectionne l'option suivante.
    pub fn select_next(&mut self) {
        self.selected_index = (self.selected_index + 1).min(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_picker_navigation_covers_mixed() {
        let oid = Oid::zero();
        let mut state = ResetPickerState::new(oid, "abc1234".to_string(), "Test".to_string());

        assert!(state.is_soft_selected());

        state.select_next();
        assert!(state.is_mixed_selected());

        state.select_next();
        assert!(state.is_hard_selected());

        state.select_previous();
        assert!(state.is_mixed_selected());
    }
}
