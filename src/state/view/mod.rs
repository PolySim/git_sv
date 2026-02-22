//! États spécifiques à chaque vue.

mod graph;
mod staging;
mod branches;
mod blame;
mod conflicts;
mod search;
mod merge_picker;

pub use graph::GraphViewState;
pub use staging::{StagingState, StagingFocus};
pub use branches::{BranchesViewState, BranchesSection, BranchesFocus, InputAction};
pub use blame::BlameState;
pub use conflicts::{ConflictsState, ConflictPanelFocus};
pub use search::SearchState;
pub use merge_picker::MergePickerState;

/// Mode de vue actif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Graph,
    Staging,
    Branches,
    Conflicts,
    Blame,
    Help,
}

/// Mode d'affichage du panneau bottom-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BottomLeftMode {
    #[default]
    Files,
    Parents,
    /// Legacy: équivalent à Files
    CommitFiles,
    /// Legacy: équivalent à Parents  
    WorkingDir,
}

impl BottomLeftMode {
    /// Bascule entre les modes.
    pub fn toggle(&mut self) {
        *self = match self {
            BottomLeftMode::Files | BottomLeftMode::CommitFiles => BottomLeftMode::Parents,
            BottomLeftMode::Parents | BottomLeftMode::WorkingDir => BottomLeftMode::Files,
        };
    }

    /// Retourne true si le mode affiche les fichiers du commit.
    pub fn is_commit_files(&self) -> bool {
        matches!(self, BottomLeftMode::Files | BottomLeftMode::CommitFiles)
    }

    /// Retourne true si le mode affiche le working directory.
    pub fn is_working_dir(&self) -> bool {
        matches!(self, BottomLeftMode::Parents | BottomLeftMode::WorkingDir)
    }
}

/// Panneau ayant le focus dans la vue principale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusPanel {
    #[default]
    Graph,
    BottomLeft,
    BottomRight,
    /// Legacy: équivalent à BottomLeft
    Files,
    /// Legacy: équivalent à BottomRight
    Detail,
}
