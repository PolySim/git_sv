//! Création de states de test.

use crate::state::selection::ListSelection;
use crate::state::{BranchesViewState, StagingState, ViewMode};

/// Builder pour créer des AppState de test.
pub struct TestStateBuilder {
    view_mode: ViewMode,
    current_branch: Option<String>,
    staged_count: usize,
    unstaged_count: usize,
}

impl Default for TestStateBuilder {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Graph,
            current_branch: Some("main".to_string()),
            staged_count: 0,
            unstaged_count: 0,
        }
    }
}

impl TestStateBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view_mode(mut self, mode: ViewMode) -> Self {
        self.view_mode = mode;
        self
    }

    pub fn branch(mut self, name: &str) -> Self {
        self.current_branch = Some(name.to_string());
        self
    }

    pub fn staged_files(mut self, count: usize) -> Self {
        self.staged_count = count;
        self
    }

    pub fn unstaged_files(mut self, count: usize) -> Self {
        self.unstaged_count = count;
        self
    }

    /// Construit un AppState minimal pour les tests.
    /// Note: Nécessite un vrai repo ou un mock selon le contexte.
    pub fn build_minimal(self) -> MinimalTestState {
        MinimalTestState {
            view_mode: self.view_mode,
            current_branch: self.current_branch,
            staging_state: StagingState::default(),
            branches_view_state: BranchesViewState::default(),
        }
    }
}

/// État minimal pour les tests unitaires (sans repo réel).
pub struct MinimalTestState {
    pub view_mode: ViewMode,
    pub current_branch: Option<String>,
    pub staging_state: StagingState,
    pub branches_view_state: BranchesViewState,
}
