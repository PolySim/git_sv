//! Etat partage des champs de texte editables.

use std::ops::Range;

const MAX_UNDO_STEPS: usize = 100;
pub(crate) const MAX_HISTORY_BYTES: usize = 8 * 1024 * 1024;

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
        self.enforce_budget(true);
    }

    /// Borne aussi le poids des instantanés, notamment après un amendement volumineux.
    pub(crate) fn enforce_budget(&mut self, keep_undo: bool) {
        let mut bytes = self
            .undo
            .iter()
            .chain(&self.redo)
            .map(|snapshot| snapshot.text.capacity())
            .sum::<usize>();
        let (older, recent) = if keep_undo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        for snapshots in [older, recent] {
            while bytes > MAX_HISTORY_BYTES && !snapshots.is_empty() {
                bytes -= snapshots.remove(0).text.capacity();
            }
        }
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
