//! Point d'entrée de l'application : initialisation et lancement.
//!
//! Ce module crée l'`App`, charge les données git initiales
//! (graphe, statut, staging), configure le terminal et délègue
//! la boucle événementielle à l'`EventHandler`.

use crate::error::Result;
use crate::git::repo::GitRepo;
use crate::terminal::TerminalSession;

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

    /// Crée une application avec les raccourcis et commandes utilisateur.
    pub fn new_with_config(
        repo: GitRepo,
        repo_path: String,
        config: &crate::config::AppConfig,
    ) -> Result<Self> {
        let mut app = Self::new(repo, repo_path)?;
        app.state.apply_config(config);
        Ok(app)
    }

    /// Lance l'application.
    pub fn run(mut self) -> Result<()> {
        let mut session = TerminalSession::setup()?;
        self.state.image_preview.initialize();

        let mut handler = crate::handler::EventHandler::new(self.state)?;
        let result = handler.run(&mut session);

        match (result, session.restore()) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(err), Ok(())) => Err(err),
            (Ok(()), Err(err)) => Err(err),
            (Err(err), Err(_restore_err)) => Err(err),
        }
    }
}
