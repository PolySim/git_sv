//! État de la vue blame.

/// État de la vue blame.
#[derive(Debug, Clone)]
pub struct BlameState {
    /// Fichier actuellement "blâmé".
    pub file_path: String,
    /// Commit Oid du commit à partir duquel on fait le blame.
    pub commit_oid: git2::Oid,
    /// Résultat du blame.
    pub blame: Option<crate::git::blame::FileBlame>,
    /// Ligne sélectionnée (0-indexed).
    pub selected_line: usize,
    /// Offset de scroll.
    pub scroll_offset: usize,
}

impl BlameState {
    pub fn new(file_path: String, commit_oid: git2::Oid) -> Self {
        Self {
            file_path,
            commit_oid,
            blame: None,
            selected_line: 0,
            scroll_offset: 0,
        }
    }
}
