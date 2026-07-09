//! Etat partage des champs de texte editables.

use std::ops::Range;

const MAX_UNDO_STEPS: usize = 100;

/// Instantane d'un champ texte utilise par l'annulation et le retablissement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextSnapshot {
    pub(crate) text: String,
    pub(crate) cursor: usize,
    pub(crate) selection_anchor: Option<usize>,
}

/// Historique d'edition borne d'un champ texte.
#[derive(Debug, Clone, Default)]
pub struct TextEditHistory {
    pub(crate) undo: Vec<TextSnapshot>,
    pub(crate) redo: Vec<TextSnapshot>,
}

impl TextEditHistory {
    /// Efface l'historique lorsqu'un nouveau champ est ouvert.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub(crate) fn record(&mut self, snapshot: TextSnapshot) {
        if self.undo.last() == Some(&snapshot) {
            return;
        }

        self.undo.push(snapshot);
        if self.undo.len() > MAX_UNDO_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}

/// Retourne la plage selectionnee en indices de caracteres.
pub fn selection_range(cursor: usize, anchor: Option<usize>) -> Option<Range<usize>> {
    let anchor = anchor?;
    if anchor == cursor {
        return None;
    }

    Some(anchor.min(cursor)..anchor.max(cursor))
}
