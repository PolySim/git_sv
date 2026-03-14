//! Gestion des opérations en arrière-plan (async).
//!
//! Utilise std::sync::mpsc pour communiquer entre le thread d'opération
//! et le thread principal de l'application.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::git::conflict::MergeResult;
use crate::git::remote::{FetchSuccess, PushSuccess};

/// Résultat d'une opération en arrière-plan.
#[derive(Debug)]
pub enum BackgroundResult {
    /// Push terminé
    Push(Result<PushSuccess, String>),
    /// Pull terminé
    Pull(Result<MergeResult, String>),
    /// Fetch terminé
    Fetch(Result<FetchSuccess, String>),
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
            let _ = tx.send(BackgroundResult::Push(result.map_err(|e| e.to_string())));
        });
    }

    /// Lance un pull en arrière-plan.
    pub fn spawn_pull(&self, repo_path: PathBuf, _branch_name: String) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            let result = crate::git::remote::pull_current_branch_cli_path(&repo_path)
                .map_err(|e| e.to_string());
            let _ = tx.send(BackgroundResult::Pull(result));
        });
    }

    /// Lance un fetch en arrière-plan.
    pub fn spawn_fetch(&self, repo_path: PathBuf) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            let result = crate::git::remote::fetch_all_cli_path(&repo_path);
            let _ = tx.send(BackgroundResult::Fetch(result.map_err(|e| e.to_string())));
        });
    }

    /// Vérifie si un résultat est disponible (non bloquant).
    pub fn try_recv(&self) -> Option<BackgroundResult> {
        self.receiver.try_recv().ok()
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
    use crate::git::remote::types::{flash_message_for_pull_result, FetchSuccess, PushSuccess};

    #[test]
    fn test_structured_background_result_variants() {
        let push = BackgroundResult::Push(Ok(PushSuccess {
            branch_name: "main".to_string(),
            remote_name: "origin".to_string(),
            force: false,
            upstream_set: false,
        }));
        let pull = BackgroundResult::Pull(Ok(MergeResult::UpToDate));
        let fetch = BackgroundResult::Fetch(Ok(FetchSuccess {
            remote_name: "origin".to_string(),
        }));

        assert!(matches!(push, BackgroundResult::Push(Ok(_))));
        assert!(matches!(
            pull,
            BackgroundResult::Pull(Ok(MergeResult::UpToDate))
        ));
        assert!(matches!(fetch, BackgroundResult::Fetch(Ok(_))));
    }

    #[test]
    fn test_background_runner_creation() {
        let runner = BackgroundRunner::new();
        // Aucun résultat ne devrait être disponible immédiatement
        assert!(runner.try_recv().is_none());
    }

    #[test]
    fn test_background_result_pull_complete() {
        let result = BackgroundResult::Pull(Ok(MergeResult::UpToDate));
        assert!(matches!(
            result,
            BackgroundResult::Pull(Ok(MergeResult::UpToDate))
        ));

        let result = BackgroundResult::Pull(Ok(MergeResult::Conflicts(Vec::new())));
        assert!(matches!(
            result,
            BackgroundResult::Pull(Ok(MergeResult::Conflicts(_)))
        ));
    }

    #[test]
    fn test_flash_message_for_structured_pull_result() {
        assert_eq!(
            flash_message_for_pull_result(&MergeResult::FastForward),
            Some(crate::utils::flash_success("Pull (fast-forward) réussi"))
        );
        assert_eq!(
            flash_message_for_pull_result(&MergeResult::Conflicts(Vec::new())),
            None
        );
    }
}
