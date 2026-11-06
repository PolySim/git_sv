//! Surveillance des changements Git sans bloquer la boucle de rendu.
//!
//! Un worker unique collecte les références et le working tree. Les demandes
//! simultanées sont fusionnées et seul le dernier résultat est conservé.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use git2::{Repository, StatusOptions};

use crate::error::Result;
use crate::git::repo::StatusEntry;

/// Intervalle de vérification des métadonnées Git (2 secondes).
const GIT_CHECK_INTERVAL: Duration = Duration::from_secs(2);
/// Intervalle du scan plus coûteux du working tree (5 secondes).
const WORKTREE_CHECK_INTERVAL: Duration = Duration::from_secs(5);
/// Fréquence de récupération des résultats pendant un scan asynchrone.
const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(50);
/// Délai de debounce après un changement détecté (500ms).
const DEBOUNCE_DELAY: Duration = Duration::from_millis(500);

/// Surveillant Git dont les scans coûteux s'exécutent hors du thread TUI.
pub struct GitWatcher {
    worker: Arc<WorkerControl>,
    last_git_check: Instant,
    last_worktree_check: Instant,
    last_change_detected: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ScanRequest {
    git: bool,
    worktree: bool,
    publish_status: bool,
}

impl ScanRequest {
    const ALL: Self = Self {
        git: true,
        worktree: true,
        publish_status: true,
    };

    fn any(self) -> bool {
        self.git || self.worktree
    }
}

#[derive(Default)]
struct ScanOutcome {
    changed: bool,
    status_entries: Option<Vec<StatusEntry>>,
    defer_status: bool,
    status_failed: bool,
    status_error: Option<String>,
}

#[derive(Default)]
struct ScanCompletion {
    git_checked: Option<Instant>,
    worktree_checked: Option<Instant>,
    changed_at: Option<Instant>,
}

struct WorkerControl {
    state: Mutex<WorkerState>,
    wake: Condvar,
}

#[derive(Default)]
struct WorkerState {
    stopped: bool,
    generation: u64,
    requested: ScanRequest,
    running: ScanRequest,
    completed: ScanCompletion,
    status: Option<StatusSnapshot>,
    status_error: Option<StatusError>,
}

struct StatusSnapshot {
    generation: u64,
    entries: Vec<StatusEntry>,
    ready: bool,
}

struct StatusError {
    generation: u64,
    message: String,
}

struct RepositorySnapshot {
    repo: Repository,
    head_mtime: Option<SystemTime>,
    index_mtime: Option<SystemTime>,
    references: Option<Vec<ReferenceSnapshot>>,
    packed_refs_mtime: Option<SystemTime>,
    worktree: Option<Vec<WorktreeEntrySnapshot>>,
    status_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ReferenceSnapshot {
    name: Vec<u8>,
    target: Option<git2::Oid>,
    symbolic_target: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorktreeEntrySnapshot {
    path: PathBuf,
    status: u32,
    modified: Option<SystemTime>,
    size: Option<u64>,
}

impl GitWatcher {
    /// Crée le surveillant et lance la collecte initiale du working tree en arrière-plan.
    ///
    /// # Erreurs
    /// Retourne une erreur si le repository ou le thread de surveillance est indisponible.
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        // libgit2 respecte les fichiers .git des worktrees et sous-modules.
        let repo = Repository::discover(repo_path)?;
        let mut snapshot = RepositorySnapshot::new(repo);
        Self::with_scanner(move |request| snapshot.scan(request))
    }

    fn with_scanner<F>(scanner: F) -> Result<Self>
    where
        F: FnMut(ScanRequest) -> ScanOutcome + Send + 'static,
    {
        let worker = Arc::new(WorkerControl {
            state: Mutex::new(WorkerState {
                requested: ScanRequest::ALL,
                ..WorkerState::default()
            }),
            wake: Condvar::new(),
        });
        let worker_control = Arc::clone(&worker);
        thread::Builder::new()
            .name("git-watcher".into())
            .spawn(move || run_worker(worker_control, scanner))?;
        Ok(Self {
            worker,
            last_git_check: Instant::now(),
            last_worktree_check: Instant::now(),
            last_change_detected: None,
        })
    }

    /// Planifie les scans échus et récupère leurs résultats sans attendre les entrées/sorties.
    /// Retourne `true` après le debounce d'un changement observé.
    pub fn check_changed(&mut self) -> Result<bool> {
        let mut state = self.worker.state.lock().unwrap();
        let completed = std::mem::take(&mut state.completed);
        if let Some(checked) = completed.git_checked {
            self.last_git_check = checked;
        }
        if let Some(checked) = completed.worktree_checked {
            self.last_worktree_check = checked;
        }
        if let Some(changed_at) = completed.changed_at {
            // Une rafale ou un reset ne doit pas repousser une détection déjà en attente.
            self.last_change_detected.get_or_insert(changed_at);
        }
        state.requested.git |=
            self.last_git_check.elapsed() >= GIT_CHECK_INTERVAL && !state.running.git;
        state.requested.worktree |= self.last_worktree_check.elapsed() >= WORKTREE_CHECK_INTERVAL
            && !state.running.worktree;
        if state.requested.any() {
            self.worker.wake.notify_one();
        }
        if self
            .last_change_detected
            .is_some_and(|changed_at| changed_at.elapsed() >= DEBOUNCE_DELAY)
        {
            self.last_change_detected = None;
            let generation = state.generation;
            if let Some(status) = &mut state.status {
                status.ready |= status.generation == generation;
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// Planifie un nouveau scan après un rafraîchissement ou une opération Git.
    /// Les résultats antérieurs sont invalidés sans supprimer les changements détectés.
    pub fn reset(&mut self) -> Result<()> {
        let mut state = self.worker.state.lock().unwrap();
        state.generation = state.generation.wrapping_add(1);
        state.requested = ScanRequest::ALL;
        self.worker.wake.notify_one();
        Ok(())
    }

    /// Récupère le dernier statut collecté sans refaire de scan sur le thread TUI.
    /// Un résultat commencé avant le dernier reset n'est jamais retourné.
    /// Les changements périodiques sont disponibles après `check_changed() == true` ;
    /// le résultat initial et les resets explicites n'attendent pas le debounce.
    pub fn take_status_entries(&mut self) -> Option<Vec<StatusEntry>> {
        let mut state = self.worker.state.lock().unwrap();
        if state
            .status
            .as_ref()
            .is_some_and(|status| status.ready && status.generation == state.generation)
        {
            return state.status.take().map(|status| status.entries);
        }
        None
    }

    /// Récupère une erreur du scan courant sans exposer de chemins ni de contenu Git.
    pub fn take_status_error(&mut self) -> Option<String> {
        let mut state = self.worker.state.lock().unwrap();
        if state
            .status_error
            .as_ref()
            .is_some_and(|error| error.generation == state.generation)
        {
            return state.status_error.take().map(|error| error.message);
        }
        None
    }

    /// Retourne l'échéance du prochain scan, résultat ou debounce à traiter.
    pub fn next_check_in(&self) -> Duration {
        let state = self.worker.state.lock().unwrap();
        if state.completed.git_checked.is_some()
            || state.completed.worktree_checked.is_some()
            || state
                .status
                .as_ref()
                .is_some_and(|status| status.ready && status.generation == state.generation)
            || state
                .status_error
                .as_ref()
                .is_some_and(|error| error.generation == state.generation)
        {
            return Duration::ZERO;
        }
        let mut delay = if state.running.any() || state.requested.any() {
            WORKER_POLL_INTERVAL
        } else {
            Duration::MAX
        };
        if !state.running.git && !state.requested.git {
            delay = delay.min(GIT_CHECK_INTERVAL.saturating_sub(self.last_git_check.elapsed()));
        }
        if !state.running.worktree && !state.requested.worktree {
            delay = delay
                .min(WORKTREE_CHECK_INTERVAL.saturating_sub(self.last_worktree_check.elapsed()));
        }
        if let Some(changed_at) = self.last_change_detected {
            delay = delay.min(DEBOUNCE_DELAY.saturating_sub(changed_at.elapsed()));
        }
        delay
    }
}

impl Drop for GitWatcher {
    fn drop(&mut self) {
        let mut state = self.worker.state.lock().unwrap();
        state.stopped = true;
        state.requested = ScanRequest::default();
        self.worker.wake.notify_one();
        // Un scan libgit2 en cours termine puis libère ses ressources ; quitter ne l'attend pas.
    }
}

fn run_worker<F>(worker: Arc<WorkerControl>, mut scanner: F)
where
    F: FnMut(ScanRequest) -> ScanOutcome,
{
    loop {
        let (request, generation) = {
            let mut state = worker.state.lock().unwrap();
            while !state.stopped && !state.requested.any() {
                state = worker.wake.wait(state).unwrap();
            }
            if state.stopped {
                return;
            }
            let request = std::mem::take(&mut state.requested);
            state.running = request;
            (request, state.generation)
        };
        // Aucun verrou partagé n'est conservé pendant les accès Git ou au disque.
        let outcome = scanner(request);
        let mut state = worker.state.lock().unwrap();
        if state.stopped {
            return;
        }
        state.running = ScanRequest::default();
        let completed_at = Instant::now();
        if request.git {
            state.completed.git_checked = Some(completed_at);
        }
        if request.worktree {
            state.completed.worktree_checked = Some(completed_at);
        }
        if outcome.changed {
            state.completed.changed_at.get_or_insert(completed_at);
        }
        if generation == state.generation && request.worktree {
            if !outcome.status_failed {
                state.status_error = None;
            } else if let Some(message) = outcome.status_error {
                state.status_error = Some(StatusError {
                    generation,
                    message,
                });
            }
        }
        let previous_status = if generation == state.generation {
            outcome.status_entries.and_then(|entries| {
                state.status.replace(StatusSnapshot {
                    generation,
                    entries,
                    ready: !outcome.defer_status,
                })
            })
        } else {
            None
        };
        drop(state);
        // Libérer les anciennes listes sur le worker, hors verrou partagé avec la TUI.
        drop(previous_status);
    }
}

impl RepositorySnapshot {
    fn new(repo: Repository) -> Self {
        Self {
            head_mtime: get_mtime(&repo.path().join("HEAD")),
            index_mtime: get_mtime(&repo.path().join("index")),
            // Ce petit état initial évite de perdre une référence modifiée pendant le premier scan.
            references: collect_reference_snapshot(&repo).ok(),
            packed_refs_mtime: get_mtime(&repo.path().join("packed-refs")),
            worktree: None,
            status_failed: false,
            repo,
        }
    }

    fn scan(&mut self, request: ScanRequest) -> ScanOutcome {
        let mut outcome = ScanOutcome::default();
        if request.git {
            outcome.changed = self.scan_git();
        }
        if request.worktree {
            // Une erreur transitoire conserve le dernier état valide et sera retentée.
            match collect_worktree_snapshot(&self.repo) {
                Ok((snapshot, entries)) => {
                    self.accept_worktree(request, snapshot, entries, &mut outcome)
                }
                Err(_) => self.report_status_error(request, &mut outcome),
            }
        }
        outcome
    }

    fn accept_worktree(
        &mut self,
        request: ScanRequest,
        snapshot: Vec<WorktreeEntrySnapshot>,
        entries: Vec<StatusEntry>,
        outcome: &mut ScanOutcome,
    ) {
        let changed = self.worktree.as_ref().is_some_and(|old| *old != snapshot);
        outcome.changed |= changed;
        if changed || self.worktree.is_none() || self.status_failed || request.publish_status {
            outcome.status_entries = Some(entries);
            outcome.defer_status = changed && !request.publish_status;
        }
        self.worktree = Some(snapshot);
        self.status_failed = false;
    }

    fn report_status_error(&mut self, request: ScanRequest, outcome: &mut ScanOutcome) {
        outcome.status_failed = true;
        if !self.status_failed || request.publish_status {
            outcome.status_error = Some(
                crate::i18n::text(
                    "Impossible de vérifier le statut du dépôt. Une nouvelle tentative suivra.",
                    "Unable to check repository status. Another attempt will follow.",
                )
                .to_string(),
            );
        }
        self.status_failed = true;
    }

    fn scan_git(&mut self) -> bool {
        let head = get_mtime(&self.repo.path().join("HEAD"));
        let index = get_mtime(&self.repo.path().join("index"));
        let packed_refs = get_mtime(&self.repo.path().join("packed-refs"));
        let mut changed = head != self.head_mtime
            || index != self.index_mtime
            || packed_refs != self.packed_refs_mtime;
        self.head_mtime = head;
        self.index_mtime = index;
        self.packed_refs_mtime = packed_refs;
        // Le mtime du dossier ne change pas lorsqu'une référence existante avance.
        if let Ok(references) = collect_reference_snapshot(&self.repo) {
            changed |= self.references.as_ref() != Some(&references);
            self.references = Some(references);
        }
        changed
    }
}

fn collect_reference_snapshot(repo: &Repository) -> Result<Vec<ReferenceSnapshot>> {
    let mut snapshot = Vec::new();
    for reference in repo.references()? {
        let reference = reference?;
        snapshot.push(ReferenceSnapshot {
            name: reference.name_bytes().to_vec(),
            target: reference.target(),
            symbolic_target: reference.symbolic_target_bytes().map(<[u8]>::to_vec),
        });
    }
    snapshot.sort_unstable();
    Ok(snapshot)
}

/// Récupère le timestamp de dernière modification d'un fichier ou répertoire.
fn get_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn collect_worktree_snapshot(
    repo: &Repository,
) -> Result<(Vec<WorktreeEntrySnapshot>, Vec<StatusEntry>)> {
    // Un index mis en cache peut masquer une erreur de relecture après une mutation externe.
    let repo = Repository::discover(repo.workdir().unwrap_or(repo.path()))?;
    let Some(root) = repo.workdir() else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .update_index(false);
    let statuses = repo.statuses(Some(&mut options))?;
    let mut snapshot = Vec::with_capacity(statuses.len());
    let mut entries = Vec::with_capacity(statuses.len());
    for entry in statuses.iter() {
        let path = PathBuf::from(String::from_utf8_lossy(entry.path_bytes()).into_owned());
        let metadata = fs::symlink_metadata(root.join(&path)).ok();
        snapshot.push(WorktreeEntrySnapshot {
            path,
            status: entry.status().bits(),
            modified: metadata.as_ref().and_then(|value| value.modified().ok()),
            size: metadata.as_ref().map(fs::Metadata::len),
        });
        entries.push(StatusEntry {
            path: entry.path().unwrap_or("???").to_string(),
            status: entry.status(),
        });
    }
    snapshot.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((snapshot, entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{self, Receiver, SyncSender};
    use tempfile::TempDir;

    fn create_repository() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        let tracked = temp_dir.path().join("tracked.txt");
        fs::write(&tracked, "initial\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("tracked.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "Initial", &tree, &[])
            .unwrap();
        temp_dir
    }

    fn force_poll(watcher: &mut GitWatcher) {
        watcher.last_git_check = Instant::now() - GIT_CHECK_INTERVAL - Duration::from_millis(1);
        watcher.last_worktree_check =
            Instant::now() - WORKTREE_CHECK_INTERVAL - Duration::from_millis(1);
    }

    fn finish_scans(watcher: &mut GitWatcher) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut changed = false;
        loop {
            changed |= watcher.check_changed().unwrap();
            let state = watcher.worker.state.lock().unwrap();
            if !state.running.any() && !state.requested.any() {
                drop(state);
                return changed | watcher.check_changed().unwrap();
            }
            drop(state);
            assert!(
                Instant::now() < deadline,
                "Le worker doit terminer son scan"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn initialized_watcher(path: &Path) -> GitWatcher {
        let mut watcher = GitWatcher::new(path).unwrap();
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.take_status_entries().is_some());
        watcher
    }

    fn controlled_watcher() -> (GitWatcher, Receiver<ScanRequest>, SyncSender<ScanOutcome>) {
        let (started, requests) = mpsc::sync_channel(1);
        let (complete, outcomes) = mpsc::sync_channel(1);
        let watcher = GitWatcher::with_scanner(move |request| {
            started.send(request).unwrap();
            outcomes.recv().unwrap_or_default()
        })
        .unwrap();
        (watcher, requests, complete)
    }

    fn status_outcome(path: &str, changed: bool) -> ScanOutcome {
        ScanOutcome {
            changed,
            status_entries: Some(vec![StatusEntry {
                path: path.to_string(),
                status: git2::Status::WT_MODIFIED,
            }]),
            ..ScanOutcome::default()
        }
    }

    #[test]
    #[ignore = "Mesure en lecture seule ; définir GIT_SV_WATCHER_BENCH_REPO"]
    fn benchmark_watcher_on_repository() {
        let path = std::env::var_os("GIT_SV_WATCHER_BENCH_REPO")
            .expect("GIT_SV_WATCHER_BENCH_REPO requis");
        let started = Instant::now();
        let mut watcher = GitWatcher::new(Path::new(&path)).unwrap();
        println!(
            "watcher_new_ms={:.3}",
            started.elapsed().as_secs_f64() * 1_000.0
        );
        let started = Instant::now();
        watcher.reset().unwrap();
        println!(
            "watcher_reset_ms={:.3}",
            started.elapsed().as_secs_f64() * 1_000.0
        );
        force_poll(&mut watcher);
        let started = Instant::now();
        watcher.check_changed().unwrap();
        println!(
            "watcher_poll_ms={:.3}",
            started.elapsed().as_secs_f64() * 1_000.0
        );
        let started = Instant::now();
        finish_scans(&mut watcher);
        assert!(watcher.take_status_entries().is_some());
        println!(
            "watcher_async_scan_ms={:.3}",
            started.elapsed().as_secs_f64() * 1_000.0
        );
    }

    #[test]
    fn watcher_reports_initial_status_without_invalidating_history() {
        let temp = create_repository();
        fs::write(temp.path().join("tracked.txt"), "changed\n").unwrap();
        let mut watcher = GitWatcher::new(temp.path()).unwrap();
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.last_change_detected.is_none());
        let entries = watcher.take_status_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "tracked.txt");
        assert!(entries[0].status.is_wt_modified());
    }

    #[test]
    fn unchanged_poll_does_not_republish_status_but_reset_does() {
        let temp = create_repository();
        let mut watcher = initialized_watcher(temp.path());
        force_poll(&mut watcher);
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.take_status_entries().is_none());
        assert!(watcher.last_change_detected.is_none());
        watcher.reset().unwrap();
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.take_status_entries().unwrap().is_empty());
    }

    #[test]
    fn watcher_rejects_missing_repository() {
        let temp = TempDir::new().unwrap();
        assert!(GitWatcher::new(temp.path()).is_err());
    }

    #[test]
    fn watcher_detects_worktree_modification_without_index_change() {
        let temp = create_repository();
        let mut watcher = initialized_watcher(temp.path());
        let index_mtime = get_mtime(&temp.path().join(".git/index"));
        fs::write(temp.path().join("tracked.txt"), "modified content\n").unwrap();
        force_poll(&mut watcher);
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.last_change_detected.is_some());
        assert_eq!(index_mtime, get_mtime(&temp.path().join(".git/index")));
        assert!(watcher.take_status_entries().is_none());
        watcher.last_change_detected = Some(Instant::now() - DEBOUNCE_DELAY);
        assert!(watcher.check_changed().unwrap());
        assert!(watcher.take_status_entries().unwrap()[0]
            .status
            .is_wt_modified());
    }

    #[test]
    fn watcher_detects_untracked_file() {
        let temp = create_repository();
        let mut watcher = initialized_watcher(temp.path());
        fs::write(temp.path().join("new.txt"), "new\n").unwrap();
        force_poll(&mut watcher);
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.last_change_detected.is_some());
        assert!(watcher.take_status_entries().is_none());
        watcher.last_change_detected = Some(Instant::now() - DEBOUNCE_DELAY);
        assert!(watcher.check_changed().unwrap());
        let entries = watcher.take_status_entries().unwrap();
        assert_eq!(entries[0].path, "new.txt");
        assert!(entries[0].status.is_wt_new());
    }

    #[test]
    fn debounce_is_not_postponed_or_cleared_by_reset() {
        let temp = create_repository();
        let mut watcher = initialized_watcher(temp.path());
        watcher.last_change_detected = Some(Instant::now() - DEBOUNCE_DELAY);
        watcher.reset().unwrap();
        assert!(watcher.check_changed().unwrap());
    }

    #[test]
    fn worktree_scan_uses_its_own_slower_interval() {
        let temp = create_repository();
        let mut watcher = initialized_watcher(temp.path());
        fs::write(temp.path().join("tracked.txt"), "modified content\n").unwrap();
        watcher.last_git_check = Instant::now() - GIT_CHECK_INTERVAL - Duration::from_millis(1);
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.last_change_detected.is_none());
        assert!(watcher.take_status_entries().is_none());
        watcher.last_worktree_check =
            Instant::now() - WORKTREE_CHECK_INTERVAL - Duration::from_millis(1);
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.last_change_detected.is_some());
        assert!(watcher.take_status_entries().is_none());
        watcher.last_change_detected = Some(Instant::now() - DEBOUNCE_DELAY);
        assert!(watcher.check_changed().unwrap());
        assert!(watcher.take_status_entries().is_some());
    }

    #[test]
    fn next_check_honors_pending_debounce() {
        let temp = create_repository();
        let mut watcher = initialized_watcher(temp.path());
        watcher.last_change_detected = Some(Instant::now() - Duration::from_millis(400));
        assert!(watcher.next_check_in() <= Duration::from_millis(100));
    }

    #[test]
    fn detects_existing_nested_and_remote_reference_updates() {
        let temp = create_repository();
        let repo = Repository::open(temp.path()).unwrap();
        let old = repo.head().unwrap().target().unwrap();
        let new = crate::git::tests::test_utils::commit_file(&repo, "next.txt", "next", "next");
        for name in [
            "refs/heads/feature/nested",
            "refs/remotes/origin/main",
            "refs/tags/test",
        ] {
            repo.reference(name, old, true, "test").unwrap();
            let mut watcher = initialized_watcher(temp.path());
            repo.reference(name, new, true, "test").unwrap();
            force_poll(&mut watcher);
            assert!(!finish_scans(&mut watcher));
            assert!(watcher.last_change_detected.is_some(), "{name}");
        }
    }

    #[test]
    fn linked_worktree_uses_own_head_and_shared_references() {
        let temp = create_repository();
        let repo = Repository::open(temp.path()).unwrap();
        let linked = temp.path().join("linked");
        repo.worktree("linked", &linked, None).unwrap();
        let linked_repo = Repository::open(&linked).unwrap();
        assert_ne!(linked_repo.path(), repo.path());
        let mut watcher = initialized_watcher(&linked);
        let head = repo.head().unwrap().target().unwrap();
        repo.reference("refs/heads/shared", head, true, "test")
            .unwrap();
        force_poll(&mut watcher);
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.last_change_detected.is_some());
        watcher.last_change_detected = None;
        linked_repo.set_head_detached(head).unwrap();
        force_poll(&mut watcher);
        assert!(!finish_scans(&mut watcher));
        assert!(watcher.last_change_detected.is_some());
    }

    #[test]
    fn slow_scan_never_blocks_polling_and_resets_are_coalesced() {
        let (mut watcher, requests, complete) = controlled_watcher();
        assert_eq!(
            requests.recv_timeout(Duration::from_secs(2)).unwrap(),
            ScanRequest::ALL
        );
        for _ in 0..100 {
            watcher.reset().unwrap();
            force_poll(&mut watcher);
            assert!(!watcher.check_changed().unwrap());
            assert!(watcher.next_check_in() > Duration::ZERO);
            assert!(watcher.next_check_in() <= WORKER_POLL_INTERVAL);
        }
        complete.send(ScanOutcome::default()).unwrap();
        assert_eq!(
            requests.recv_timeout(Duration::from_secs(2)).unwrap(),
            ScanRequest::ALL
        );
        complete.send(ScanOutcome::default()).unwrap();
        assert!(!finish_scans(&mut watcher));
        assert!(requests.try_recv().is_err());
        assert!(watcher.next_check_in() > WORKER_POLL_INTERVAL);
    }

    #[test]
    fn reset_discards_old_status_but_preserves_changes_detected_during_scan() {
        let (mut watcher, requests, complete) = controlled_watcher();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        watcher.reset().unwrap();
        complete.send(status_outcome("obsolete.txt", true)).unwrap();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!watcher.check_changed().unwrap());
        assert!(watcher.take_status_entries().is_none());
        assert!(watcher.last_change_detected.is_some());
        complete.send(status_outcome("current.txt", false)).unwrap();
        assert!(!finish_scans(&mut watcher));
        assert_eq!(
            watcher.take_status_entries().unwrap()[0].path,
            "current.txt"
        );
        assert!(watcher.last_change_detected.is_some());
        watcher.last_change_detected = Some(Instant::now() - DEBOUNCE_DELAY);
        assert!(watcher.check_changed().unwrap());
    }

    #[test]
    fn reset_discards_already_completed_status() {
        let (mut watcher, requests, complete) = controlled_watcher();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        complete
            .send(status_outcome("obsolete.txt", false))
            .unwrap();
        finish_scans(&mut watcher);
        watcher.reset().unwrap();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        // Le résultat initial ne peut pas être réappliqué après une mutation Git.
        assert!(watcher.take_status_entries().is_none());
        complete.send(status_outcome("current.txt", false)).unwrap();
        finish_scans(&mut watcher);
        assert_eq!(
            watcher.take_status_entries().unwrap()[0].path,
            "current.txt"
        );
    }

    #[test]
    fn status_errors_respect_reset_generation_and_are_consumed_once() {
        let (mut watcher, requests, complete) = controlled_watcher();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        watcher.reset().unwrap();
        complete
            .send(ScanOutcome {
                status_failed: true,
                status_error: Some("obsolete error".to_string()),
                ..ScanOutcome::default()
            })
            .unwrap();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        watcher.check_changed().unwrap();
        assert!(watcher.take_status_error().is_none());
        complete
            .send(ScanOutcome {
                status_failed: true,
                status_error: Some("current error".to_string()),
                ..ScanOutcome::default()
            })
            .unwrap();
        finish_scans(&mut watcher);
        assert_eq!(watcher.next_check_in(), Duration::ZERO);
        assert_eq!(
            watcher.take_status_error().as_deref(),
            Some("current error")
        );
        assert!(watcher.take_status_error().is_none());
        assert!(watcher.next_check_in() > WORKER_POLL_INTERVAL);
    }

    #[test]
    fn completed_results_are_bounded_to_the_latest_snapshot() {
        let (mut watcher, requests, complete) = controlled_watcher();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        complete.send(status_outcome("first.txt", false)).unwrap();
        finish_scans(&mut watcher);
        force_poll(&mut watcher);
        watcher.check_changed().unwrap();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        complete.send(status_outcome("latest.txt", true)).unwrap();
        finish_scans(&mut watcher);
        assert_eq!(watcher.take_status_entries().unwrap()[0].path, "latest.txt");
        assert!(watcher.take_status_entries().is_none());
    }

    struct ExitSignal(SyncSender<()>);

    impl Drop for ExitSignal {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    #[test]
    fn dropping_watcher_during_scan_cancels_queued_work_and_releases_worker() {
        let (started, requests) = mpsc::sync_channel(1);
        let (complete, outcomes) = mpsc::sync_channel(1);
        let (exited, exit) = mpsc::sync_channel(1);
        let exit_signal = ExitSignal(exited);
        let mut watcher = GitWatcher::with_scanner(move |request| {
            let _keep_until_worker_exits = &exit_signal;
            started.send(request).unwrap();
            outcomes.recv().unwrap_or_default()
        })
        .unwrap();
        requests.recv_timeout(Duration::from_secs(2)).unwrap();
        watcher.reset().unwrap();
        drop(watcher);
        complete.send(ScanOutcome::default()).unwrap();
        exit.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn recovered_status_is_published_even_when_worktree_is_unchanged() {
        let temp = create_repository();
        let repo = Repository::open(temp.path()).unwrap();
        let mut snapshot = RepositorySnapshot::new(repo);
        assert!(snapshot.scan(ScanRequest::ALL).status_entries.is_some());
        let request = ScanRequest {
            worktree: true,
            ..ScanRequest::default()
        };
        let index_path = temp.path().join(".git/index");
        let saved_index = fs::read(&index_path).unwrap();
        fs::write(&index_path, b"invalid index").unwrap();
        assert!(snapshot.scan(request).status_error.is_some());

        fs::write(&index_path, saved_index).unwrap();
        let recovered = snapshot.scan(request);
        assert!(!recovered.changed);
        assert!(!recovered.defer_status);
        assert!(!recovered.status_failed);
        assert!(recovered.status_error.is_none());
        assert!(recovered.status_entries.unwrap().is_empty());
        assert!(snapshot.scan(request).status_entries.is_none());
    }

    #[test]
    fn failed_status_scan_preserves_baseline_until_repository_recovers() {
        let temp = create_repository();
        let repo = Repository::open(temp.path()).unwrap();
        let mut snapshot = RepositorySnapshot::new(repo);
        assert!(snapshot.scan(ScanRequest::ALL).status_entries.is_some());
        let index_path = temp.path().join(".git/index");
        let saved_index = fs::read(&index_path).unwrap();
        fs::write(&index_path, b"invalid index").unwrap();
        let outcome = snapshot.scan(ScanRequest {
            worktree: true,
            ..ScanRequest::default()
        });
        assert!(!outcome.changed);
        assert!(outcome.status_entries.is_none());
        assert!(outcome.status_failed);
        assert!(outcome.status_error.is_some());
        let repeated = snapshot.scan(ScanRequest {
            worktree: true,
            ..ScanRequest::default()
        });
        assert!(repeated.status_failed);
        assert!(repeated.status_error.is_none());
        fs::write(&index_path, saved_index).unwrap();
        fs::write(temp.path().join("tracked.txt"), "recovered change\n").unwrap();
        let outcome = snapshot.scan(ScanRequest::ALL);
        assert!(outcome.changed);
        assert!(!outcome.status_failed);
        assert!(outcome.status_error.is_none());
        assert!(outcome.status_entries.unwrap()[0].status.is_wt_modified());
    }
}
