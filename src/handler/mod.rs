//! Gestionnaires d'événements et d'actions.
//!
//! Ce module remplace event.rs par un système modulaire de handlers.
//! Chaque handler spécialisé gère un domaine fonctionnel spécifique.

#![allow(unused_imports)]

pub mod background;
pub mod branch;
pub mod conflict;
pub mod dispatcher;
pub mod edit;
pub mod filter;
pub mod git;
pub mod navigation;
pub mod search;
pub mod staging;
pub mod traits;

// Re-exports des handlers et dispatcher
pub use background::{BackgroundResult, BackgroundRunner};
pub use dispatcher::ActionDispatcher;
pub use traits::{ActionHandler, HandlerContext};

use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;

use crate::error::Result;
use crate::state::{AppState, ViewMode};
use crate::ui;
use crate::ui::input::handle_input_with_timeout;
use crate::watcher::GitWatcher;

/// Gestionnaire principal de la boucle événementielle.
///
/// Remplace l'ancien event::EventHandler par une version utilisant
/// le dispatcher modulaire pour router les actions.
pub struct EventHandler {
    state: AppState,
    dispatcher: ActionDispatcher,
    watcher: GitWatcher,
    background: BackgroundRunner,
}

impl EventHandler {
    /// Crée un nouveau gestionnaire d'événements.
    pub fn new(state: AppState) -> Result<Self> {
        let watcher = GitWatcher::new(&state.repo_path)?;
        Ok(Self {
            state,
            dispatcher: ActionDispatcher::new(),
            watcher,
            background: BackgroundRunner::new(),
        })
    }

    /// Lance la boucle événementielle principale.
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        // Rafraîchissement initial si nécessaire
        if self.state.dirty {
            self.refresh()?;
        }

        loop {
            // Rendu (avec spinner si une opération est en cours)
            terminal.draw(|frame| {
                ui::render(frame, &mut self.state);
                // Si un spinner est actif, le rendre en overlay
                if let Some(spinner) = &mut self.state.loading_spinner {
                    crate::ui::loading::render_overlay(frame, spinner, frame.area());
                }
            })?;

            // Vérifier les résultats d'opérations en arrière-plan
            if let Some(result) = self.background.try_recv() {
                self.handle_background_result(result)?;
            }

            // Vérifier les changements dans le repository git (auto-refresh)
            if self.watcher.check_changed()? {
                self.state.dirty = true;
            }

            // Input avec timeout adaptatif
            // Timeout court (80ms) quand le spinner est actif pour une animation fluide
            let timeout_ms = if self.state.loading_spinner.is_some() {
                80
            } else if self.state.flash_message.is_some() {
                100
            } else {
                250
            };

            let action = handle_input_with_timeout(&self.state, timeout_ms)?;

            // Pendant le loading, ignorer les actions sauf Quit
            if self.state.loading_spinner.is_some() {
                match action {
                    Some(crate::state::AppAction::Quit) => {
                        self.state.should_quit = true;
                    }
                    _ => {
                        // Ignorer les autres inputs pendant le chargement
                    }
                }
            } else if let Some(action) = action {
                // Passer le background runner aux handlers qui en ont besoin
                self.dispatch_with_background(action)?;
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

    /// Dispatche une action avec accès au background runner si nécessaire.
    fn dispatch_with_background(&mut self, action: crate::state::AppAction) -> Result<()> {
        use crate::state::action::GitAction;

        match action {
            crate::state::AppAction::Git(GitAction::Push) => {
                self.handle_push_background()?;
            }
            crate::state::AppAction::Git(GitAction::ForcePush) => {
                self.handle_force_push_background()?;
            }
            crate::state::AppAction::Git(GitAction::Pull) => {
                self.handle_pull_background()?;
            }
            crate::state::AppAction::Git(GitAction::Fetch) => {
                self.handle_fetch_background()?;
            }
            _ => {
                // Actions normales sans background
                self.dispatcher.dispatch(&mut self.state, action)?;
            }
        }
        Ok(())
    }

    /// Lance un push en arrière-plan avec spinner.
    fn handle_push_background(&mut self) -> Result<()> {
        // Activer le spinner
        self.state.loading_spinner = Some(crate::ui::loading::LoadingSpinner::new(
            "Push en cours...".to_string(),
        ));

        // Lancer l'opération en arrière-plan
        let repo_path = std::path::PathBuf::from(&self.state.repo_path);
        self.background.spawn_push(repo_path, false);

        Ok(())
    }

    /// Lance un force push en arrière-plan avec spinner.
    fn handle_force_push_background(&mut self) -> Result<()> {
        self.state.loading_spinner = Some(crate::ui::loading::LoadingSpinner::new(
            "Force push en cours...".to_string(),
        ));

        let repo_path = std::path::PathBuf::from(&self.state.repo_path);
        self.background.spawn_push(repo_path, true);

        Ok(())
    }

    /// Lance un pull en arrière-plan avec spinner.
    fn handle_pull_background(&mut self) -> Result<()> {
        // Activer le spinner
        self.state.loading_spinner = Some(crate::ui::loading::LoadingSpinner::new(
            "Pull en cours...".to_string(),
        ));

        // Lancer l'opération en arrière-plan
        let repo_path = std::path::PathBuf::from(&self.state.repo_path);
        let branch_name = self.state.current_branch.clone().unwrap_or_default();
        self.background.spawn_pull(repo_path, branch_name);

        Ok(())
    }

    /// Lance un fetch en arrière-plan avec spinner.
    fn handle_fetch_background(&mut self) -> Result<()> {
        // Activer le spinner
        self.state.loading_spinner = Some(crate::ui::loading::LoadingSpinner::new(
            "Fetch en cours...".to_string(),
        ));

        // Lancer l'opération en arrière-plan
        let repo_path = std::path::PathBuf::from(&self.state.repo_path);
        self.background.spawn_fetch(repo_path);

        Ok(())
    }

    /// Traite le résultat d'une opération en arrière-plan.
    fn handle_background_result(&mut self, result: BackgroundResult) -> Result<()> {
        // Désactiver le spinner
        self.state.loading_spinner = None;

        match result {
            BackgroundResult::PushComplete(Ok(msg)) => {
                self.state.set_flash_message(msg);
                self.state.mark_dirty();
            }
            BackgroundResult::PushComplete(Err(err)) => {
                self.state
                    .set_flash_message(format!("Erreur push : {}", err));
            }
            BackgroundResult::PullComplete(Ok(msg)) => {
                self.state.set_flash_message(msg);
                self.state.mark_dirty();
            }
            BackgroundResult::PullComplete(Err(err)) => {
                self.state
                    .set_flash_message(format!("Erreur pull : {}", err));
            }
            BackgroundResult::FetchComplete(Ok(msg)) => {
                self.state.set_flash_message(msg);
                self.state.mark_dirty();
            }
            BackgroundResult::FetchComplete(Err(err)) => {
                self.state
                    .set_flash_message(format!("Erreur fetch : {}", err));
            }
        }
        Ok(())
    }

    /// Rafraîchit les données depuis le repository.
    fn refresh(&mut self) -> Result<()> {
        // Mise à jour des données de base
        self.state.current_branch = self.state.repo.current_branch().ok();

        // Construire le graphe avec ou sans filtres
        self.state.graph = if self.state.graph_filter.is_active() {
            self.state
                .repo
                .build_graph_filtered(crate::state::MAX_COMMITS, &self.state.graph_filter)
                .unwrap_or_default()
        } else {
            self.state
                .repo
                .build_graph(crate::state::MAX_COMMITS)
                .unwrap_or_default()
        };

        self.state.status_entries = self.state.repo.status().unwrap_or_default();

        // Synchroniser graph_view.rows avec le graphe reconstruit
        self.state
            .graph_view
            .rows
            .set_items(self.state.graph.clone());

        // Synchronisation de la sélection - ne pas dépasser les bornes
        if self.state.selected_index >= self.state.graph.len() && !self.state.graph.is_empty() {
            self.state.selected_index = self.state.graph.len() - 1;
        }
        if self.state.graph.is_empty() {
            self.state.selected_index = 0;
        }

        // Synchroniser graph_view avec la nouvelle sélection
        self.state.graph_view.rows.select(self.state.selected_index);

        // Synchroniser graph_state (ListState de ratatui) avec la sélection
        // Le graphe contient 2 items par commit (ligne + connexion)
        self.state
            .graph_state
            .select(Some(self.state.selected_index * 2));

        // Clamper file_selected_index pour éviter les index hors limites
        if self.state.file_selected_index >= self.state.commit_files.len() {
            self.state.file_selected_index = self.state.commit_files.len().saturating_sub(1);
        }

        // Mise à jour des fichiers du commit sélectionné
        if let Some(row) = self.state.graph.get(self.state.selected_index) {
            self.state.commit_files = self
                .state
                .repo
                .commit_diff(row.node.oid)
                .unwrap_or_default();
        } else {
            self.state.commit_files.clear();
        }

        // Mise à jour du staging
        let all_entries = self.state.repo.status().unwrap_or_default();
        self.state.staging_state.set_staged_files(
            all_entries
                .iter()
                .filter(|e| e.is_staged())
                .cloned()
                .collect(),
        );
        self.state.staging_state.set_unstaged_files(
            all_entries
                .iter()
                .filter(|e| e.is_unstaged())
                .cloned()
                .collect(),
        );

        // Charger les données de la vue branches
        if self.state.view_mode == ViewMode::Branches {
            match crate::git::branch::list_all_branches(&self.state.repo.repo) {
                Ok((local, remote)) => {
                    self.state
                        .branches_view_state
                        .local_branches
                        .set_items(local);
                    self.state
                        .branches_view_state
                        .remote_branches
                        .set_items(remote);
                }
                Err(e) => {
                    self.state
                        .set_flash_message(format!("Erreur chargement branches: {}", e));
                }
            }

            // Charger les worktrees
            match crate::git::worktree::list_worktrees(&self.state.repo.repo) {
                Ok(worktrees) => {
                    self.state
                        .branches_view_state
                        .worktrees
                        .set_items(worktrees);
                }
                Err(_) => {}
            }

            // Charger les stashes
            match crate::git::stash::list_stashes(&mut self.state.repo.repo) {
                Ok(stashes) => {
                    self.state.branches_view_state.stashes.set_items(stashes);
                }
                Err(_) => {}
            }
        }

        // Charger le diff si on est en mode Staging
        if self.state.view_mode == ViewMode::Staging {
            staging::load_staging_diff(&mut self.state);
        }

        // Détecter si un merge est en cours
        self.state.is_merging = crate::git::conflict::is_merging(&self.state.repo.repo);

        // Réinitialiser le flag dirty
        self.state.dirty = false;

        // Réinitialiser le watcher pour éviter de redétecter les mêmes changements
        self.watcher.reset()?;

        Ok(())
    }
}
