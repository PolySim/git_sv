//! Gestionnaires d'événements et d'actions.
//!
//! Ce module remplace event.rs par un système modulaire de handlers.
//! Chaque handler spécialisé gère un domaine fonctionnel spécifique.

pub mod background;
pub mod branch;
pub mod conflict;
pub mod dispatcher;
pub mod edit;
pub mod filter;
pub mod git;
pub mod navigation;
pub mod project_tree;
pub mod search;
pub mod staging;
pub mod traits;

// Re-exports des handlers et dispatcher
pub use background::{BackgroundResult, BackgroundRunner};
pub use dispatcher::ActionDispatcher;

use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;
use std::time::Duration;

use crate::error::Result;
use crate::state::AppState;
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

        let mut needs_redraw = true;
        loop {
            if needs_redraw {
                terminal.draw(|frame| {
                    ui::render(frame, &mut self.state);
                    if let Some(spinner) = &mut self.state.ui.loading_spinner {
                        crate::ui::loading::render_overlay(frame, spinner, frame.area());
                    }
                })?;
                needs_redraw = false;
            }

            let input = handle_input_with_timeout(&self.state, self.next_wake_in())?;
            needs_redraw |= input.event_received;

            // Pendant le loading, ignorer les actions sauf Quit
            if self.state.ui.loading_spinner.is_some() {
                match input.action {
                    Some(crate::state::AppAction::Quit) => {
                        self.state.request_quit();
                    }
                    _ => {
                        // Ignorer les autres inputs pendant le chargement
                    }
                }
            } else if let Some(action) = input.action {
                // Passer le background runner aux handlers qui en ont besoin
                self.dispatch_with_background(action)?;
            }

            if self.state.ui.should_quit {
                break;
            }

            // Vérifier les résultats d'opérations en arrière-plan
            if let Some(result) = self.background.try_recv() {
                self.handle_background_result(result)?;
                needs_redraw = true;
            }

            // Vérifier les changements dans le repository git (auto-refresh)
            if self.watcher.check_changed()? {
                self.state.mark_dirty();
            }

            // Vérifier si le message flash a expiré
            needs_redraw |= self.state.check_flash_expired();

            // Rafraîchissement conditionnel
            if self.state.dirty {
                self.refresh()?;
                needs_redraw = true;
            }

            // Le spinner est le seul élément nécessitant une animation continue.
            needs_redraw |= self.state.ui.loading_spinner.is_some();
        }
        Ok(())
    }

    fn next_wake_in(&self) -> Duration {
        if self.state.ui.loading_spinner.is_some() {
            return Duration::from_millis(80);
        }

        let mut timeout = self.watcher.next_check_in();
        if let Some(flash_delay) = self.state.ui.flash_expiry_in() {
            timeout = timeout.min(flash_delay);
        }
        timeout.max(Duration::from_millis(1))
    }

    /// Dispatche une action avec accès au background runner si nécessaire.
    fn dispatch_with_background(&mut self, action: crate::state::AppAction) -> Result<()> {
        use crate::state::action::GitAction;

        let previous_repo_path = self.state.repo_path.clone();

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
        if self.state.repo_path != previous_repo_path {
            self.watcher = GitWatcher::new(&self.state.repo_path)?;
        }
        Ok(())
    }

    /// Lance un push en arrière-plan avec spinner.
    fn handle_push_background(&mut self) -> Result<()> {
        self.state.set_loading("Push en cours...");

        // Lancer l'opération en arrière-plan
        let repo_path = std::path::PathBuf::from(&self.state.repo_path);
        self.background.spawn_push(repo_path, false);

        Ok(())
    }

    /// Lance un force push en arrière-plan avec spinner.
    fn handle_force_push_background(&mut self) -> Result<()> {
        self.state.set_loading("Force push en cours...");

        let repo_path = std::path::PathBuf::from(&self.state.repo_path);
        self.background.spawn_push(repo_path, true);

        Ok(())
    }

    /// Lance un pull en arrière-plan avec spinner.
    fn handle_pull_background(&mut self) -> Result<()> {
        self.state.set_loading("Pull en cours...");

        // Lancer l'opération en arrière-plan
        let repo_path = std::path::PathBuf::from(&self.state.repo_path);
        let branch_name = self.state.current_branch.clone().unwrap_or_default();
        self.background.spawn_pull(repo_path, branch_name);

        Ok(())
    }

    /// Lance un fetch en arrière-plan avec spinner.
    fn handle_fetch_background(&mut self) -> Result<()> {
        self.state.set_loading("Fetch en cours...");

        // Lancer l'opération en arrière-plan
        let repo_path = std::path::PathBuf::from(&self.state.repo_path);
        self.background.spawn_fetch(repo_path);

        Ok(())
    }

    /// Traite le résultat d'une opération en arrière-plan.
    fn handle_background_result(&mut self, result: BackgroundResult) -> Result<()> {
        use crate::git::conflict::MergeResult;
        use crate::git::remote::flash_message_for_pull_result;
        use crate::state::ConflictsState;

        self.state.clear_loading();

        match result {
            BackgroundResult::Push(Ok(success)) => {
                self.state.set_flash_message(success.flash_message());
                self.state.mark_dirty();
            }
            BackgroundResult::Push(Err(err)) => {
                self.state
                    .set_flash_message(crate::utils::flash_error("push", err));
            }
            BackgroundResult::Pull(Ok(pull_result)) => match pull_result {
                MergeResult::Conflicts(files) => {
                    if files.is_empty() {
                        // Détecter les fichiers en conflit dans le thread principal
                        match crate::git::conflict::list_conflict_files(&self.state.repo.repo) {
                            Ok(detected_files) => {
                                let ours_name = crate::git::conflict::get_current_branch_name(
                                    &self.state.repo.repo,
                                );
                                let theirs_name = format!(
                                    "origin/{}",
                                    self.state
                                        .current_branch
                                        .clone()
                                        .unwrap_or_else(|| "HEAD".to_string())
                                );
                                self.state.open_conflicts(ConflictsState::new(
                                    detected_files,
                                    "Pull depuis origin".to_string(),
                                    ours_name,
                                    theirs_name,
                                ));
                                self.state.set_flash_message(
                                    "Conflits lors du pull - résolution requise".to_string(),
                                );
                            }
                            Err(e) => {
                                self.state.set_flash_message(crate::utils::flash_error(
                                    "détection conflits",
                                    e,
                                ));
                            }
                        }
                    } else {
                        let ours_name =
                            crate::git::conflict::get_current_branch_name(&self.state.repo.repo);
                        let theirs_name = format!(
                            "origin/{}",
                            self.state
                                .current_branch
                                .clone()
                                .unwrap_or_else(|| "HEAD".to_string())
                        );
                        self.state.open_conflicts(ConflictsState::new(
                            files,
                            "Pull depuis origin".to_string(),
                            ours_name,
                            theirs_name,
                        ));
                        self.state.set_flash_message(
                            "Conflits lors du pull - résolution requise".to_string(),
                        );
                    }
                }
                other => {
                    if let Some(message) = flash_message_for_pull_result(&other) {
                        self.state.set_flash_message(message);
                    }
                    if !matches!(other, MergeResult::UpToDate) {
                        self.state.mark_dirty();
                    }
                }
            },
            BackgroundResult::Pull(Err(err)) => {
                self.state
                    .set_flash_message(crate::utils::flash_error("pull", err));
            }
            BackgroundResult::Fetch(Ok(success)) => {
                self.state.set_flash_message(success.flash_message());
                self.state.mark_dirty();
            }
            BackgroundResult::Fetch(Err(err)) => {
                self.state
                    .set_flash_message(crate::utils::flash_error("fetch", err));
            }
        }
        Ok(())
    }

    /// Rafraîchit les données depuis le repository.
    ///
    /// Utilise l'API unifiée de GraphViewState pour garantir la cohérence de l'état.
    fn refresh(&mut self) -> Result<()> {
        self.state.refresh_from_repo()?;

        // Réinitialiser le watcher pour éviter de redétecter les mêmes changements
        self.watcher.reset()?;

        Ok(())
    }
}
