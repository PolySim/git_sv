//! Gestion des opérations en arrière-plan (async).
//!
//! Utilise std::sync::mpsc pour communiquer entre le thread d'opération
//! et le thread principal de l'application.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Résultat d'une opération en arrière-plan.
#[derive(Debug)]
pub enum BackgroundResult {
    /// Push terminé (message de succès ou erreur)
    PushComplete(Result<String, String>),
    /// Pull terminé (message de succès ou erreur)
    PullComplete(Result<String, String>),
    /// Fetch terminé (message de succès ou erreur)
    FetchComplete(Result<String, String>),
}

/// Gestionnaire des opérations en arrière-plan.
pub struct BackgroundRunner {
    /// Canal pour recevoir les résultats des opérations
    pub receiver: Receiver<BackgroundResult>,
    /// Canal pour envoyer les résultats (cloné pour chaque thread)
    sender: Sender<BackgroundResult>,
}

impl BackgroundRunner {
    /// Crée un nouveau gestionnaire d'opérations en arrière-plan.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    /// Lance un push en arrière-plan.
    pub fn spawn_push(&self, repo_path: PathBuf) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            // Utiliser la version CLI car git2::Repository n'est pas Send
            let result = crate::git::remote::push_current_branch_cli_path(&repo_path);
            let _ = tx.send(BackgroundResult::PushComplete(
                result
                    .map(|_| "Push réussi ✓".to_string())
                    .map_err(|e| e.to_string()),
            ));
        });
    }

    /// Lance un pull en arrière-plan.
    pub fn spawn_pull(&self, repo_path: PathBuf, _branch_name: String) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            // Utiliser la version CLI car git2::Repository n'est pas Send
            let result = crate::git::remote::pull_current_branch_cli_path(&repo_path);
            let _ = tx.send(BackgroundResult::PullComplete(
                result
                    .map(|_| "Pull réussi ✓".to_string())
                    .map_err(|e| e.to_string()),
            ));
        });
    }

    /// Lance un fetch en arrière-plan.
    pub fn spawn_fetch(&self, repo_path: PathBuf) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            // Utiliser la version CLI car git2::Repository n'est pas Send
            let result = crate::git::remote::fetch_all_cli_path(&repo_path);
            let _ = tx.send(BackgroundResult::FetchComplete(
                result
                    .map(|_| "Fetch réussi ✓".to_string())
                    .map_err(|e| e.to_string()),
            ));
        });
    }

    /// Vérifie si un résultat est disponible (non bloquant).
    pub fn try_recv(&self) -> Option<BackgroundResult> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(_) => None,
        }
    }
}

impl Default for BackgroundRunner {
    fn default() -> Self {
        Self::new()
    }
}
