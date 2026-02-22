use crate::error::Result;
use crate::git::repo::GitRepo;
use crate::terminal::{restore_terminal, setup_terminal};

/// Application principale qui orchestre les composants.
pub struct App {
    state: crate::state::AppState,
}

impl App {
    /// Crée une nouvelle instance de l'application.
    pub fn new(repo: GitRepo, repo_path: String) -> Result<Self> {
        let mut state = crate::state::AppState::new(repo, repo_path)?;

        // Rafraîchir l'état initial.
        state.current_branch = state.repo.current_branch().ok();
        state.graph = state
            .repo
            .build_graph(crate::state::MAX_COMMITS)
            .unwrap_or_default();
        state.status_entries = state.repo.status().unwrap_or_default();

        // Charger les données initiales.
        if let Some(row) = state.graph.get(state.selected_index) {
            state.commit_files = state.repo.commit_diff(row.node.oid).unwrap_or_default();
        }

        // Rafraîchir l'état de staging.
        let all_entries = state.repo.status().unwrap_or_default();
        state.staging_state.staged_files = all_entries
            .iter()
            .filter(|e| e.is_staged())
            .cloned()
            .collect();
        state.staging_state.unstaged_files = all_entries
            .iter()
            .filter(|e| e.is_unstaged())
            .cloned()
            .collect();

        Ok(Self { state })
    }

    /// Lance l'application.
    pub fn run(self) -> Result<()> {
        let mut terminal = setup_terminal()?;

        let mut handler = crate::event::EventHandler::new(self.state);
        let result = handler.run(&mut terminal);

        restore_terminal(&mut terminal)?;
        result
    }
}

// Ré-export des types publiquement utilisés
pub use crate::state::{
    BottomLeftMode, BranchesFocus, BranchesSection, BranchesViewState, InputAction, StagingFocus,
    StagingState,
};
