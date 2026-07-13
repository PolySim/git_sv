//! États spécifiques à chaque vue.

mod blame;
mod branches;
mod conflicts;
mod graph;
mod merge_picker;
mod project_tree;
mod reset_picker;
mod search;
mod staging;

pub use blame::BlameState;
pub use branches::{
    BranchesFocus, BranchesSection, BranchesViewState, InputAction, SelectedBranch,
};
pub use conflicts::{ConflictPanelFocus, ConflictsState};
pub use graph::GraphViewState;
pub use merge_picker::{BranchPickerMode, MergePickerState};
pub use project_tree::{ProjectEntryKind, ProjectTreeFocus, ProjectTreeState};
pub use reset_picker::ResetPickerState;
pub use search::SearchState;
pub use staging::{StagingFocus, StagingState};

/// Mode de vue actif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Graph,
    Staging,
    Branches,
    ProjectTree,
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
}

impl BottomLeftMode {
    /// Bascule entre les modes.
    pub fn toggle(&mut self) {
        *self = match self {
            BottomLeftMode::Files => BottomLeftMode::Parents,
            BottomLeftMode::Parents => BottomLeftMode::Files,
        };
    }
}

/// Panneau ayant le focus dans la vue principale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusPanel {
    #[default]
    Graph,
    BottomLeft,
    BottomRight,
}
