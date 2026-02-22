//! Gestionnaires d'événements et d'actions.
//!
//! Ce module remplace event.rs par un système modulaire de handlers.
//! Chaque handler spécialisé gère un domaine fonctionnel spécifique.

pub mod traits;
pub mod navigation;
pub mod staging;
pub mod git;
pub mod branch;
pub mod conflict;
pub mod search;
pub mod edit;
pub mod dispatcher;

// Re-exports des handlers et dispatcher
pub use traits::{ActionHandler, HandlerContext};
pub use navigation::NavigationHandler;
pub use staging::StagingHandler;
pub use git::GitHandler;
pub use branch::BranchHandler;
pub use conflict::ConflictHandler;
pub use search::SearchHandler;
pub use edit::EditHandler;
pub use dispatcher::ActionDispatcher;

use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;

use crate::error::Result;
use crate::state::AppState;
use crate::ui;
use crate::ui::input::handle_input_with_timeout;

/// Gestionnaire principal de la boucle événementielle.
///
/// Remplace l'ancien event::EventHandler par une version utilisant
/// le dispatcher modulaire pour router les actions.
pub struct EventHandler {
    state: AppState,
    dispatcher: ActionDispatcher,
}

impl EventHandler {
    /// Crée un nouveau gestionnaire d'événements.
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            dispatcher: ActionDispatcher::new(),
        }
    }

    /// Lance la boucle événementielle principale.
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        // Rafraîchissement initial si nécessaire
        if self.state.dirty {
            self.refresh()?;
        }

        loop {
            // Rendu
            terminal.draw(|frame| {
                ui::render(frame, &self.state);
            })?;

            // Input avec timeout adaptatif
            let timeout_ms = if self.state.flash_message.is_some() {
                100
            } else {
                250
            };

            if let Some(action) = handle_input_with_timeout(&self.state, timeout_ms)? {
                self.dispatcher.dispatch(&mut self.state, action)?;
            }

            if self.state.should_quit {
                break;
            }

            // Vérifier si le message flash a expiré
            self.state.check_flash_expired();

            // Rafraîchissement conditionnel
            if self.state.dirty {
                self.refresh()?;
            }
        }
        Ok(())
    }

    /// Rafraîchit les données depuis le repository.
    fn refresh(&mut self) -> Result<()> {
        // Mise à jour des données de base
        self.state.current_branch = self.state.repo.current_branch().ok();
        self.state.graph = self.state.repo.build_graph(crate::state::MAX_COMMITS).unwrap_or_default();
        self.state.status_entries = self.state.repo.status().unwrap_or_default();

        // Synchronisation de la sélection
        if self.state.selected_index >= self.state.graph.len() && !self.state.graph.is_empty() {
            self.state.selected_index = self.state.graph.len() - 1;
        }

        // Mise à jour des fichiers du commit sélectionné
        if let Some(row) = self.state.graph.get(self.state.selected_index) {
            self.state.commit_files = self.state.repo.commit_diff(row.node.oid).unwrap_or_default();
        }

        // Mise à jour du staging
        let all_entries = self.state.repo.status().unwrap_or_default();
        self.state.staging_state.set_staged_files(
            all_entries.iter().filter(|e| e.is_staged()).cloned().collect()
        );
        self.state.staging_state.set_unstaged_files(
            all_entries.iter().filter(|e| e.is_unstaged()).cloned().collect()
        );

        // Réinitialiser le flag dirty
        self.state.dirty = false;

        Ok(())
    }
}
