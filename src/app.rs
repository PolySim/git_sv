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

        state.initialize_from_repo()?;

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
