//! État de la vue de recherche.

use crate::git::search::SearchType;
use crate::state::TextEditHistory;

/// État de la recherche de commits.
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// Recherche activée ou non.
    pub is_active: bool,
    /// Texte de recherche.
    pub query: String,
    /// Position du curseur dans le texte de recherche.
    pub cursor: usize,
    /// Point d'ancrage de la selection, en indices de caracteres.
    pub selection_anchor: Option<usize>,
    /// Historique d'annulation de la recherche.
    pub edit_history: TextEditHistory,
    /// Type de recherche en cours.
    pub search_type: SearchType,
    /// Indices des commits correspondant à la recherche.
    pub results: Vec<usize>,
    /// Index du résultat actuellement sélectionné dans results.
    pub current_result: usize,
}

impl SearchState {
    /// Ouvre la recherche.
    pub fn open(&mut self) {
        self.is_active = true;
        self.query.clear();
        self.cursor = 0;
        self.selection_anchor = None;
        self.edit_history.clear();
        self.results.clear();
        self.current_result = 0;
    }

    /// Ferme la recherche.
    pub fn close(&mut self) {
        self.is_active = false;
    }

    /// Passe au résultat suivant.
    pub fn next_result(&mut self) {
        if !self.results.is_empty() {
            self.current_result = (self.current_result + 1) % self.results.len();
        }
    }

    /// Passe au résultat précédent.
    pub fn previous_result(&mut self) {
        if !self.results.is_empty() {
            self.current_result = if self.current_result == 0 {
                self.results.len() - 1
            } else {
                self.current_result - 1
            };
        }
    }

    /// Change le type de recherche.
    pub fn cycle_search_type(&mut self) {
        self.search_type = match self.search_type {
            SearchType::Message => SearchType::Author,
            SearchType::Author => SearchType::Hash,
            SearchType::Hash => SearchType::Message,
        };
    }
}
