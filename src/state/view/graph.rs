//! État de la vue graph.

use crate::git::graph::GraphRow;
use crate::state::selection::ListSelection;

/// État de la vue graph avec gestion de sélection générique.
#[derive(Debug, Clone, Default)]
pub struct GraphViewState {
    /// Lignes du graph avec sélection.
    pub rows: ListSelection<GraphRow>,
    /// Index du fichier sélectionné dans le commit courant.
    pub file_selected_index: usize,
    /// Offset de scroll dans le diff.
    pub diff_scroll_offset: usize,
}

impl GraphViewState {
    /// Crée un nouvel état graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Commit actuellement sélectionné.
    pub fn selected_commit(&self) -> Option<&crate::git::graph::CommitNode> {
        self.rows.selected_item().map(|row| &row.node)
    }
}
