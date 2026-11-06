//! Chargement de l'historique hors du thread TUI, avec une seule demande en attente.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use git2::{Oid, Repository};

use crate::git::commit::CommitInfo;
use crate::git::project_tree::{compare_path_history_at, path_history_at, PathHistoryComparison};

use super::MAX_TOTAL_COMMITS;

/// Identité complète d'une demande, y compris les références figées au départ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectHistoryRequest {
    pub repo_path: String,
    pub path: String,
    pub is_directory: bool,
    pub generation: u64,
    pub head: Option<Oid>,
    pub comparison: Option<(String, Oid)>,
}

pub(super) enum ProjectHistoryData {
    History(Vec<CommitInfo>),
    Comparison(PathHistoryComparison),
}

pub(super) struct ProjectHistoryResult {
    pub request: ProjectHistoryRequest,
    pub data: std::result::Result<ProjectHistoryData, String>,
}

#[derive(Default)]
struct WorkerState {
    stopped: bool,
    running: bool,
    latest: Option<ProjectHistoryRequest>,
    queued: Option<ProjectHistoryRequest>,
    completed: Option<ProjectHistoryResult>,
}

struct WorkerControl {
    state: Mutex<WorkerState>,
    wake: Condvar,
}

pub(super) struct ProjectHistoryLoader {
    control: Arc<WorkerControl>,
}

impl ProjectHistoryLoader {
    pub fn new() -> std::io::Result<Self> {
        Self::with_loader(load_history)
    }

    pub(super) fn with_loader<F>(mut loader: F) -> std::io::Result<Self>
    where
        F: FnMut(&ProjectHistoryRequest) -> std::result::Result<ProjectHistoryData, String>
            + Send
            + 'static,
    {
        let control = Arc::new(WorkerControl {
            state: Mutex::new(WorkerState::default()),
            wake: Condvar::new(),
        });
        let worker = Arc::clone(&control);
        thread::Builder::new()
            .name("git-path-history".into())
            .spawn(move || loop {
                let request = {
                    let mut state = worker.state.lock().unwrap();
                    while !state.stopped && state.queued.is_none() {
                        state = worker.wake.wait(state).unwrap();
                    }
                    if state.stopped {
                        return;
                    }
                    state.running = true;
                    state.queued.take().unwrap()
                };
                let data = loader(&request);
                let mut state = worker.state.lock().unwrap();
                state.running = false;
                if state.stopped {
                    return;
                }
                // Un calcul dépassé ne doit ni remplacer le résultat récent ni s'accumuler.
                if state.latest.as_ref() == Some(&request) {
                    state.completed = Some(ProjectHistoryResult { request, data });
                }
            })?;
        Ok(Self { control })
    }

    pub fn request(&self, request: ProjectHistoryRequest) {
        let mut state = self.control.state.lock().unwrap();
        if state.latest.as_ref() == Some(&request) {
            return;
        }
        state.latest = Some(request.clone());
        state.queued = Some(request);
        state.completed = None;
        self.control.wake.notify_one();
    }

    pub fn take_completed(&self) -> Option<ProjectHistoryResult> {
        let mut state = self.control.state.lock().unwrap();
        let result = state.completed.take()?;
        state.latest = None;
        Some(result)
    }

    pub fn pending(&self) -> bool {
        let state = self.control.state.lock().unwrap();
        state.running || state.queued.is_some() || state.completed.is_some()
    }

    pub fn cancel(&self) {
        let mut state = self.control.state.lock().unwrap();
        state.latest = None;
        state.queued = None;
        state.completed = None;
    }
}

impl Drop for ProjectHistoryLoader {
    fn drop(&mut self) {
        let mut state = self.control.state.lock().unwrap();
        state.stopped = true;
        state.latest = None;
        state.queued = None;
        state.completed = None;
        // Le thread quitte après son calcul actuel, sans bloquer la fermeture du terminal.
        self.control.wake.notify_one();
    }
}

pub(super) fn load_history(
    request: &ProjectHistoryRequest,
) -> std::result::Result<ProjectHistoryData, String> {
    let repo = Repository::open(&request.repo_path).map_err(|error| error.to_string())?;
    match (&request.comparison, request.head) {
        (Some((_, target)), Some(head)) => compare_path_history_at(
            &repo,
            head,
            *target,
            &request.path,
            request.is_directory,
            MAX_TOTAL_COMMITS,
        )
        .map(ProjectHistoryData::Comparison)
        .map_err(|error| error.to_string()),
        (None, Some(head)) => path_history_at(
            &repo,
            head,
            &request.path,
            request.is_directory,
            MAX_TOTAL_COMMITS,
        )
        .map(ProjectHistoryData::History)
        .map_err(|error| error.to_string()),
        (None, None) => Ok(ProjectHistoryData::History(Vec::new())),
        (Some(_), None) => Err("HEAD ne contient aucun commit à comparer".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn request(generation: u64) -> ProjectHistoryRequest {
        ProjectHistoryRequest {
            repo_path: "repository".into(),
            path: format!("file-{generation}"),
            is_directory: false,
            generation,
            head: None,
            comparison: None,
        }
    }

    #[test]
    fn rapid_selections_load_only_the_running_and_latest_requests() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let loader = ProjectHistoryLoader::with_loader(move |request| {
            started_tx.send(request.generation).unwrap();
            release_rx.recv().unwrap();
            Ok(ProjectHistoryData::History(Vec::new()))
        })
        .unwrap();
        loader.request(request(0));
        assert_eq!(started_rx.recv_timeout(Duration::from_secs(2)).unwrap(), 0);
        for generation in 1..=1_000 {
            loader.request(request(generation));
        }
        release_tx.send(()).unwrap();
        assert_eq!(
            started_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            1_000
        );
        assert!(loader.take_completed().is_none());
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let completed = loop {
            if let Some(result) = loader.take_completed() {
                break result;
            }
            assert!(Instant::now() < deadline, "résultat du worker attendu");
            thread::yield_now();
        };
        assert_eq!(completed.request.generation, 1_000);
        assert!(!loader.pending());
    }

    #[test]
    fn dropping_a_busy_loader_does_not_wait_or_start_queued_work() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let loader = ProjectHistoryLoader::with_loader(move |request| {
            started_tx.send(request.generation).unwrap();
            release_rx.recv().unwrap();
            Ok(ProjectHistoryData::History(Vec::new()))
        })
        .unwrap();
        loader.request(request(0));
        assert_eq!(started_rx.recv_timeout(Duration::from_secs(2)).unwrap(), 0);
        loader.request(request(1));
        // Le worker attend toujours release : joindre son thread ici provoquerait un blocage.
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let dropper = thread::spawn(move || {
            drop(loader);
            dropped_tx.send(()).unwrap();
        });
        let dropped_without_waiting = dropped_rx.recv_timeout(Duration::from_secs(2)).is_ok();
        release_tx.send(()).unwrap();
        dropper.join().unwrap();
        assert!(dropped_without_waiting);
        assert_eq!(
            started_rx.recv_timeout(Duration::from_secs(2)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn cancellation_discards_running_results() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let loader = ProjectHistoryLoader::with_loader(move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(ProjectHistoryData::History(Vec::new()))
        })
        .unwrap();
        loader.request(request(0));
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        loader.cancel();
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while loader.pending() {
            assert!(Instant::now() < deadline, "annulation du worker attendue");
            thread::yield_now();
        }
        assert!(loader.take_completed().is_none());
    }
}
