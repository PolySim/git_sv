//! Gestion des opérations en arrière-plan (async).
//!
//! Utilise std::sync::mpsc pour communiquer entre le thread d'opération
//! et le thread principal de l'application.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Résultat détaillé d'un pull en arrière-plan.
#[derive(Debug)]
pub enum PullBackgroundResult {
    /// Déjà à jour
    UpToDate,
    /// Fast-forward réussi
    FastForward,
    /// Merge réussi
    Success,
    /// Conflits détectés (liste des fichiers en conflit)
    Conflicts(Vec<String>),
    /// Erreur (message d'erreur)
    Error(String),
}

/// Résultat d'une opération en arrière-plan.
#[derive(Debug)]
pub enum BackgroundResult {
    /// Push terminé (message de succès ou erreur)
    PushComplete(Result<String, String>),
    /// Pull terminé (résultat détaillé)
    PullComplete(PullBackgroundResult),
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
    pub fn spawn_push(&self, repo_path: PathBuf, force: bool) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            let result = if force {
                crate::git::remote::force_push_current_branch_cli_path(&repo_path)
            } else {
                crate::git::remote::push_current_branch_cli_path(&repo_path)
            };
            let _ = tx.send(BackgroundResult::PushComplete(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    /// Lance un pull en arrière-plan.
    pub fn spawn_pull(&self, repo_path: PathBuf, _branch_name: String) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            // Utiliser la version CLI car git2::Repository n'est pas Send
            match crate::git::remote::pull_current_branch_cli_path(&repo_path) {
                Ok(msg) => {
                    // Analyser le message pour déterminer le type de résultat
                    let result =
                        if msg.contains("déjà à jour") || msg.contains("Already up to date") {
                            PullBackgroundResult::UpToDate
                        } else if msg.contains("fast-forward") || msg.contains("Fast-forward") {
                            PullBackgroundResult::FastForward
                        } else {
                            PullBackgroundResult::Success
                        };
                    let _ = tx.send(BackgroundResult::PullComplete(result));
                }
                Err(e) => {
                    let err_str = e.to_string();
                    // Vérifier si c'est une erreur de conflit
                    if err_str.contains("conflit")
                        || err_str.contains("conflict")
                        || err_str.contains("CONFLIT")
                        || err_str.contains("CONFLICT")
                    {
                        // Signaler qu'il y a des conflits - la détection détaillée se fera dans le thread principal
                        let _ = tx.send(BackgroundResult::PullComplete(
                            PullBackgroundResult::Conflicts(Vec::new()),
                        ));
                    } else {
                        let _ = tx.send(BackgroundResult::PullComplete(
                            PullBackgroundResult::Error(err_str),
                        ));
                    }
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pull_background_result_variants() {
        // Vérifier que tous les variants peuvent être créés
        let up_to_date = PullBackgroundResult::UpToDate;
        let fast_forward = PullBackgroundResult::FastForward;
        let success = PullBackgroundResult::Success;
        let conflicts = PullBackgroundResult::Conflicts(vec!["file.txt".to_string()]);
        let error = PullBackgroundResult::Error("test error".to_string());

        // Vérifier le debug format
        assert!(format!("{:?}", up_to_date).contains("UpToDate"));
        assert!(format!("{:?}", fast_forward).contains("FastForward"));
        assert!(format!("{:?}", success).contains("Success"));
        assert!(format!("{:?}", conflicts).contains("Conflicts"));
        assert!(format!("{:?}", error).contains("Error"));
    }

    #[test]
    fn test_background_runner_creation() {
        let runner = BackgroundRunner::new();
        // Aucun résultat ne devrait être disponible immédiatement
        assert!(runner.try_recv().is_none());
    }

    #[test]
    fn test_background_result_pull_complete() {
        let result = BackgroundResult::PullComplete(PullBackgroundResult::UpToDate);
        assert!(matches!(
            result,
            BackgroundResult::PullComplete(PullBackgroundResult::UpToDate)
        ));

        let result = BackgroundResult::PullComplete(PullBackgroundResult::Conflicts(vec![]));
        if let BackgroundResult::PullComplete(PullBackgroundResult::Conflicts(files)) = result {
            assert!(files.is_empty());
        } else {
            panic!("Expected Conflicts variant");
        }
    }
}
