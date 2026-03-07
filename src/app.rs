//! Point d'entrée de l'application : initialisation et lancement.
//!
//! Ce module crée l'`App`, charge les données git initiales
//! (graphe, statut, staging), configure le terminal et délègue
//! la boucle événementielle à l'`EventHandler`.

use crate::error::Result;
use crate::git::repo::GitRepo;
use crate::terminal::{restore_terminal, setup_terminal};

/// Application principale qui orchestre les composants.
pub struct App {
    state: crate::state::AppState,
}

impl App {
    /// Crée une nouvelle instance de l'application.
    ///
    /// Utilise l'API unifiée de GraphViewState pour initialiser l'état
    /// de manière cohérente avec le rafraîchissement.
    pub fn new(repo: GitRepo, repo_path: String) -> Result<Self> {
        let mut state = crate::state::AppState::new(repo, repo_path)?;

        // Rafraîchir l'état initial avec l'API unifiée
        state.current_branch = state.repo.current_branch().ok();

        // Construire le graphe initial et l'assigner via l'API unifiée
        let initial_graph = state
            .repo
            .build_graph(crate::state::MAX_COMMITS)
            .unwrap_or_default();
        state.replace_graph(initial_graph);

        // Charger les fichiers du commit sélectionné
        state.refresh_commit_files();

        // Charger le diff du premier fichier si disponible
        if !state.graph_view.commit_files.is_empty() {
            crate::handler::navigation::load_commit_file_diff(&mut state);
        }

        // Statut du working directory
        state.status_entries = state.repo.status().unwrap_or_default();

        // Rafraîchir l'état de staging.
        let all_entries = state.repo.status().unwrap_or_default();
        state.staging_state.set_staged_files(
            all_entries
                .iter()
                .filter(|e| e.is_staged())
                .cloned()
                .collect(),
        );
        state.staging_state.set_unstaged_files(
            all_entries
                .iter()
                .filter(|e| e.is_unstaged())
                .cloned()
                .collect(),
        );

        Ok(Self { state })
    }

    /// Lance l'application.
    pub fn run(self) -> Result<()> {
        let mut terminal = setup_terminal()?;

        let mut handler = crate::handler::EventHandler::new(self.state)?;
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
