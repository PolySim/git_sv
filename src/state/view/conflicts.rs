//! État de la vue de résolution de conflits.

use crate::git::conflict::{ConflictResolutionMode, MergeFile};

/// Focus dans les panneaux de la vue conflits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPanelFocus {
    #[default]
    FileList,
    OursPanel,
    TheirsPanel,
    ResultPanel,
}

/// État de la vue de résolution de conflits.
#[derive(Debug, Clone)]
pub struct ConflictsState {
    /// Tous les fichiers du merge (en conflit ou non).
    pub all_files: Vec<MergeFile>,
    /// Index du fichier sélectionné.
    pub file_selected: usize,
    /// Index de la section de conflit sélectionnée.
    pub section_selected: usize,
    /// Index de la ligne sélectionnée (mode ligne).
    pub line_selected: usize,
    /// Mode de résolution actif.
    pub resolution_mode: ConflictResolutionMode,
    /// Scroll dans le panneau ours.
    pub ours_scroll: usize,
    /// Scroll dans le panneau theirs.
    pub theirs_scroll: usize,
    /// Scroll dans le panneau résultat.
    pub result_scroll: usize,
    /// Focus dans les panneaux (Ours / Theirs / Result).
    pub panel_focus: ConflictPanelFocus,
    /// Description de l'opération en cours.
    pub operation_description: String,
    /// Nom de la branche "ours" (HEAD).
    pub ours_branch_name: String,
    /// Nom de la branche "theirs" (branche mergée).
    pub theirs_branch_name: String,
    /// Mode édition actif dans le panneau résultat.
    pub is_editing: bool,
    /// Buffer éditable (contenu du résultat, modifiable).
    pub edit_buffer: Vec<String>,
    /// Ligne du curseur dans le buffer d'édition.
    pub edit_cursor_line: usize,
    /// Colonne du curseur dans le buffer d'édition.
    pub edit_cursor_col: usize,
}

impl ConflictsState {
    /// Crée un nouvel état de conflits à partir de ConflictFile (compatibilité).
    pub fn new(
        files: Vec<crate::git::conflict::ConflictFile>,
        operation_description: String,
        ours_branch_name: String,
        theirs_branch_name: String,
    ) -> Self {
        // Convertir les ConflictFile en MergeFile
        let all_files: Vec<MergeFile> = files
            .into_iter()
            .map(|f| MergeFile {
                path: f.path,
                has_conflicts: true,
                conflicts: f.conflicts,
                is_resolved: f.is_resolved,
                conflict_type: Some(f.conflict_type),
            })
            .collect();

        Self {
            all_files,
            file_selected: 0,
            section_selected: 0,
            line_selected: 0,
            resolution_mode: ConflictResolutionMode::Block,
            ours_scroll: 0,
            theirs_scroll: 0,
            result_scroll: 0,
            panel_focus: ConflictPanelFocus::FileList,
            operation_description,
            ours_branch_name,
            theirs_branch_name,
            is_editing: false,
            edit_buffer: Vec::new(),
            edit_cursor_line: 0,
            edit_cursor_col: 0,
        }
    }

    /// Fichier actuellement sélectionné.
    pub fn selected_file(&self) -> Option<&MergeFile> {
        self.all_files.get(self.file_selected)
    }

    /// Nombre de fichiers avec des conflits restants.
    pub fn remaining_conflicts(&self) -> usize {
        self.all_files
            .iter()
            .filter(|f| f.has_conflicts && !f.is_resolved)
            .count()
    }
}
