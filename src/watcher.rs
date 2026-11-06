//! Surveillance des changements dans le repository git.
//!
//! Ce module implémente une détection automatique des changements git
//! en surveillant les timestamps de modification des fichiers clés du
//! répertoire `.git/` (HEAD, index, refs/). Lorsqu'un changement est
//! détecté, un flag est levé pour signaler qu'un rafraîchissement
//! est nécessaire.
//!
//! La surveillance utilise un polling périodique avec un debounce
//! pour éviter les rafraîchissements excessifs.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use git2::{Repository, StatusOptions};

use crate::error::Result;

/// Intervalle de vérification des métadonnées Git (2 secondes).
const GIT_CHECK_INTERVAL: Duration = Duration::from_secs(2);
/// Intervalle du scan plus coûteux du working tree (5 secondes).
const WORKTREE_CHECK_INTERVAL: Duration = Duration::from_secs(5);
/// Délai de debounce après un changement détecté (500ms).
const DEBOUNCE_DELAY: Duration = Duration::from_millis(500);

/// Surveillant de changements git par polling des timestamps.
///
/// Cette structure maintient les timestamps des fichiers surveillés
/// et détecte les modifications en comparant avec les valeurs
/// précédentes.
pub struct GitWatcher {
    /// Chemin vers le répertoire `.git/`.
    git_dir: PathBuf,
    /// Timestamp de dernière vérification des métadonnées Git.
    last_git_check: Instant,
    /// Timestamp du dernier scan du working tree.
    last_worktree_check: Instant,
    /// Timestamp de dernière modification détectée (pour debounce).
    last_change_detected: Option<Instant>,
    /// Timestamp du fichier HEAD.
    head_mtime: Option<SystemTime>,
    /// Timestamp du fichier index.
    index_mtime: Option<SystemTime>,
    /// Valeurs des références, y compris celles partagées entre worktrees.
    references: Vec<ReferenceSnapshot>,
    /// Timestamp du fichier packed-refs.
    packed_refs_mtime: Option<SystemTime>,
    /// Chemin du working tree surveillé.
    worktree_dir: PathBuf,
    /// État observable des fichiers modifiés dans le working tree.
    worktree_snapshot: Vec<WorktreeEntrySnapshot>,
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
    /// Crée un nouveau surveillant pour le repository à la racine donnée.
    ///
    /// # Paramètres
    ///
    /// * `repo_path` - Chemin vers le repository git (peut être un sous-répertoire).
    ///
    /// # Erreurs
    ///
    /// Retourne une erreur si le répertoire `.git/` n'est pas trouvé.
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref();

        // libgit2 respecte les fichiers .git des worktrees et sous-modules.
        let repo = Repository::discover(repo_path)?;
        let git_dir = repo.path().to_path_buf();
        let worktree_dir = repo.workdir().unwrap_or(repo.path()).to_path_buf();

        let mut watcher = Self {
            git_dir,
            last_git_check: Instant::now(),
            last_worktree_check: Instant::now(),
            last_change_detected: None,
            head_mtime: None,
            index_mtime: None,
            references: Vec::new(),
            packed_refs_mtime: None,
            worktree_dir,
            worktree_snapshot: Vec::new(),
        };

        // Initialiser les timestamps
        watcher.update_timestamps()?;

        Ok(watcher)
    }

    /// Met à jour les timestamps des fichiers surveillés.
    fn update_git_timestamps(&mut self) -> bool {
        self.head_mtime = get_mtime(&self.git_dir.join("HEAD"));
        self.index_mtime = get_mtime(&self.git_dir.join("index"));

        // Le mtime du dossier ne change pas lorsqu'une référence existante avance.
        let mut refs_changed = false;
        if let Ok(references) = collect_reference_snapshot(&self.worktree_dir) {
            refs_changed = self.references != references;
            self.references = references;
        }
        self.packed_refs_mtime = get_mtime(&self.git_dir.join("packed-refs"));
        refs_changed
    }

    fn update_worktree_snapshot(&mut self) {
        // Une erreur Git transitoire ne doit jamais arrêter la boucle TUI.
        if let Ok(snapshot) = collect_worktree_snapshot(&self.worktree_dir) {
            self.worktree_snapshot = snapshot;
        }
    }

    fn update_timestamps(&mut self) -> Result<()> {
        self.update_git_timestamps();
        self.update_worktree_snapshot();
        Ok(())
    }

    /// Vérifie si des changements ont eu lieu depuis le dernier appel.
    ///
    /// Cette méthode doit être appelée régulièrement dans la boucle
    /// principale. Elle retourne `true` uniquement si :
    /// - L'intervalle de vérification est écoulé
    /// - Un changement est détecté
    /// - Le délai de debounce est écoulé depuis la dernière détection
    ///
    /// # Valeur de retour
    ///
    /// `true` si un rafraîchissement est nécessaire, `false` sinon.
    pub fn check_changed(&mut self) -> Result<bool> {
        // Le debounce est réévalué à chaque frame, sans attendre le prochain polling.
        if self
            .last_change_detected
            .is_some_and(|change_time| change_time.elapsed() >= DEBOUNCE_DELAY)
        {
            self.last_change_detected = None;
            return Ok(true);
        }

        let should_check_git = self.last_git_check.elapsed() >= GIT_CHECK_INTERVAL;
        let should_check_worktree = self.last_worktree_check.elapsed() >= WORKTREE_CHECK_INTERVAL;
        if !should_check_git && !should_check_worktree {
            return Ok(false);
        }

        let mut changed = false;
        if should_check_git {
            self.last_git_check = Instant::now();
            let old_head = self.head_mtime;
            let old_index = self.index_mtime;
            let old_packed_refs = self.packed_refs_mtime;

            let refs_changed = self.update_git_timestamps();
            changed |= self.head_mtime != old_head
                || self.index_mtime != old_index
                || refs_changed
                || self.packed_refs_mtime != old_packed_refs;
        }

        if should_check_worktree {
            self.last_worktree_check = Instant::now();
            if let Ok(snapshot) = collect_worktree_snapshot(&self.worktree_dir) {
                changed |= snapshot != self.worktree_snapshot;
                self.worktree_snapshot = snapshot;
            }
        }

        if changed {
            // Conserver la première détection pour éviter qu'une rafale repousse le refresh.
            self.last_change_detected.get_or_insert_with(Instant::now);
        }

        Ok(false)
    }

    /// Force une vérification immédiate sans attendre l'intervalle.
    ///
    /// Utile lors d'un rafraîchissement manuel pour réinitialiser
    /// les timestamps de référence.
    pub fn reset(&mut self) -> Result<()> {
        self.last_git_check = Instant::now();
        self.last_worktree_check = Instant::now();
        self.last_change_detected = None;
        self.update_timestamps()
    }

    /// Retourne le délai avant la prochaine vérification nécessaire.
    pub fn next_check_in(&self) -> Duration {
        if let Some(change_time) = self.last_change_detected {
            return DEBOUNCE_DELAY.saturating_sub(change_time.elapsed());
        }

        let git_delay = GIT_CHECK_INTERVAL.saturating_sub(self.last_git_check.elapsed());
        let worktree_delay =
            WORKTREE_CHECK_INTERVAL.saturating_sub(self.last_worktree_check.elapsed());
        git_delay.min(worktree_delay)
    }
}

fn collect_reference_snapshot(repo_path: &Path) -> Result<Vec<ReferenceSnapshot>> {
    let repo = Repository::discover(repo_path)?;
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

fn collect_worktree_snapshot(worktree_dir: &Path) -> Result<Vec<WorktreeEntrySnapshot>> {
    let repo = Repository::discover(worktree_dir)?;
    let root = repo.workdir().unwrap_or(worktree_dir);
    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut options))?;

    let mut snapshot = statuses
        .iter()
        .map(|entry| {
            let path = PathBuf::from(String::from_utf8_lossy(entry.path_bytes()).into_owned());
            let metadata = fs::metadata(root.join(&path)).ok();
            WorktreeEntrySnapshot {
                path,
                status: entry.status().bits(),
                modified: metadata.as_ref().and_then(|value| value.modified().ok()),
                size: metadata.as_ref().map(fs::Metadata::len),
            }
        })
        .collect::<Vec<_>>();
    snapshot.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
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

    #[test]
    fn test_watcher_creation() {
        let temp_dir = create_repository();
        assert!(GitWatcher::new(temp_dir.path()).is_ok());
    }

    #[test]
    fn test_watcher_detects_no_changes_initially() {
        let temp_dir = create_repository();
        let mut watcher = GitWatcher::new(temp_dir.path()).unwrap();
        force_poll(&mut watcher);
        assert!(!watcher.check_changed().unwrap());
        assert!(watcher.last_change_detected.is_none());
    }

    #[test]
    fn test_get_mtime_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test").unwrap();

        let mtime = get_mtime(&file_path);
        assert!(mtime.is_some());
    }

    #[test]
    fn test_get_mtime_nonexistent_file() {
        let mtime = get_mtime(Path::new("/nonexistent/path"));
        assert!(mtime.is_none());
    }

    #[test]
    fn test_watcher_detects_worktree_modification_without_index_change() {
        let temp_dir = create_repository();
        let mut watcher = GitWatcher::new(temp_dir.path()).unwrap();
        fs::write(temp_dir.path().join("tracked.txt"), "modified content\n").unwrap();

        force_poll(&mut watcher);
        assert!(!watcher.check_changed().unwrap());
        watcher.last_change_detected = Some(Instant::now() - DEBOUNCE_DELAY);

        assert!(watcher.check_changed().unwrap());
    }

    #[test]
    fn test_watcher_detects_untracked_file() {
        let temp_dir = create_repository();
        let mut watcher = GitWatcher::new(temp_dir.path()).unwrap();
        fs::write(temp_dir.path().join("new.txt"), "new\n").unwrap();

        force_poll(&mut watcher);
        assert!(!watcher.check_changed().unwrap());
        watcher.last_change_detected = Some(Instant::now() - DEBOUNCE_DELAY);

        assert!(watcher.check_changed().unwrap());
    }

    #[test]
    fn test_debounce_is_not_postponed_by_subsequent_changes() {
        let temp_dir = create_repository();
        let mut watcher = GitWatcher::new(temp_dir.path()).unwrap();
        watcher.last_change_detected = Some(Instant::now() - DEBOUNCE_DELAY);
        fs::write(temp_dir.path().join("tracked.txt"), "another change\n").unwrap();

        force_poll(&mut watcher);

        assert!(watcher.check_changed().unwrap());
    }

    #[test]
    fn test_worktree_scan_uses_its_own_slower_interval() {
        let temp_dir = create_repository();
        let mut watcher = GitWatcher::new(temp_dir.path()).unwrap();
        fs::write(temp_dir.path().join("tracked.txt"), "modified content\n").unwrap();

        watcher.last_git_check = Instant::now() - GIT_CHECK_INTERVAL - Duration::from_millis(1);
        watcher.last_worktree_check = Instant::now();
        assert!(!watcher.check_changed().unwrap());
        assert!(watcher.last_change_detected.is_none());

        watcher.last_worktree_check =
            Instant::now() - WORKTREE_CHECK_INTERVAL - Duration::from_millis(1);
        assert!(!watcher.check_changed().unwrap());
        assert!(watcher.last_change_detected.is_some());
    }

    #[test]
    fn test_next_check_honors_pending_debounce() {
        let temp_dir = create_repository();
        let mut watcher = GitWatcher::new(temp_dir.path()).unwrap();
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
            let mut watcher = GitWatcher::new(temp.path()).unwrap();
            repo.reference(name, new, true, "test").unwrap();
            force_poll(&mut watcher);
            assert!(!watcher.check_changed().unwrap());
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
        let mut watcher = GitWatcher::new(&linked).unwrap();
        assert_eq!(watcher.git_dir, linked_repo.path());
        let head = repo.head().unwrap().target().unwrap();
        repo.reference("refs/heads/shared", head, true, "test")
            .unwrap();
        force_poll(&mut watcher);
        assert!(!watcher.check_changed().unwrap());
        assert!(watcher.last_change_detected.is_some());
    }
}
