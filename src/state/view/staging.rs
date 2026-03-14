//! État de la vue staging.

use crate::git::diff::{DiffViewMode, FileDiff};
use crate::git::repo::StatusEntry;
use crate::state::selection::ListSelection;

/// Focus dans la vue staging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StagingFocus {
    #[default]
    Unstaged,
    Staged,
    Diff,
    CommitMessage,
}

/// État complet de la vue staging.
#[derive(Debug, Clone, Default)]
pub struct StagingState {
    /// Fichiers non stagés.
    pub unstaged: ListSelection<StatusEntry>,
    /// Fichiers stagés.
    pub staged: ListSelection<StatusEntry>,
    /// Panneau actif.
    pub focus: StagingFocus,
    /// Dernier panneau fichier actif avant ouverture du diff.
    pub last_file_focus: StagingFocus,
    /// Message de commit en cours.
    pub commit_message: String,
    /// Position du curseur dans le message.
    pub cursor_position: usize,
    /// Mode saisie de message activé.
    pub is_committing: bool,
    /// Mode amendement activé.
    pub is_amending: bool,
    /// Diff du fichier sélectionné.
    pub current_diff: Option<FileDiff>,
    /// Offset de scroll du diff.
    pub diff_scroll: usize,
    /// Offset de scroll horizontal du diff.
    pub diff_horizontal_offset: usize,
    /// Mode d'affichage du diff (unifié ou côte à côte).
    pub diff_view_mode: DiffViewMode,
}

impl StagingState {
    /// Crée un nouvel état staging.
    pub fn new() -> Self {
        Self::default()
    }

    // ═══════════════════════════════════════════════════
    // Compatibilité ascendante - champs legacy
    // ═══════════════════════════════════════════════════

    /// Accès aux fichiers stagés (compatibilité).
    pub fn staged_files(&self) -> &Vec<StatusEntry> {
        &self.staged.items
    }

    /// Accès mutable aux fichiers stagés (compatibilité).
    #[allow(dead_code)]
    pub fn staged_files_mut(&mut self) -> &mut Vec<StatusEntry> {
        &mut self.staged.items
    }

    /// Définit les fichiers stagés (compatibilité).
    pub fn set_staged_files(&mut self, files: Vec<StatusEntry>) {
        self.staged.set_items(files);
    }

    /// Accès aux fichiers non stagés (compatibilité).
    pub fn unstaged_files(&self) -> &Vec<StatusEntry> {
        &self.unstaged.items
    }

    /// Accès mutable aux fichiers non stagés (compatibilité).
    #[allow(dead_code)]
    pub fn unstaged_files_mut(&mut self) -> &mut Vec<StatusEntry> {
        &mut self.unstaged.items
    }

    /// Définit les fichiers non stagés (compatibilité).
    pub fn set_unstaged_files(&mut self, files: Vec<StatusEntry>) {
        self.unstaged.set_items(files);
    }

    /// Index sélectionné dans unstaged (compatibilité).
    pub fn unstaged_selected(&self) -> usize {
        self.unstaged.selected_index()
    }

    /// Index sélectionné dans staged (compatibilité).
    pub fn staged_selected(&self) -> usize {
        self.staged.selected_index()
    }

    /// Définit l'index sélectionné dans unstaged (compatibilité).
    pub fn set_unstaged_selected(&mut self, index: usize) {
        self.unstaged.select(index);
    }

    /// Définit l'index sélectionné dans staged (compatibilité).
    pub fn set_staged_selected(&mut self, index: usize) {
        self.staged.select(index);
    }
}
