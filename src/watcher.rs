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

use crate::error::Result;

/// Intervalle de vérification des changements (2 secondes).
const CHECK_INTERVAL: Duration = Duration::from_secs(2);
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
    /// Timestamp de dernière vérification.
    last_check: Instant,
    /// Timestamp de dernière modification détectée (pour debounce).
    last_change_detected: Option<Instant>,
    /// Timestamp du fichier HEAD.
    head_mtime: Option<SystemTime>,
    /// Timestamp du fichier index.
    index_mtime: Option<SystemTime>,
    /// Timestamp du répertoire refs/heads.
    refs_mtime: Option<SystemTime>,
}

impl GitWatcher {
    /// Crée un nouveau surveillant pour le repository à la racine donnée.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Chemin vers le repository git (peut être un sous-répertoire).
    ///
    /// # Returns
    ///
    /// Retourne une erreur si le répertoire `.git/` n'est pas trouvé.
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref();

        // Trouver le répertoire .git/ (peut être directement ou dans un parent)
        let git_dir = find_git_dir(repo_path)?;

        let mut watcher = Self {
            git_dir,
            last_check: Instant::now(),
            last_change_detected: None,
            head_mtime: None,
            index_mtime: None,
            refs_mtime: None,
        };

        // Initialiser les timestamps
        watcher.update_timestamps()?;

        Ok(watcher)
    }

    /// Met à jour les timestamps des fichiers surveillés.
    fn update_timestamps(&mut self) -> Result<()> {
        self.head_mtime = get_mtime(&self.git_dir.join("HEAD"));
        self.index_mtime = get_mtime(&self.git_dir.join("index"));

        // Le répertoire refs/heads contient les références des branches
        let refs_heads = self.git_dir.join("refs").join("heads");
        self.refs_mtime = get_mtime(&refs_heads);

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
    /// # Returns
    ///
    /// `true` si un rafraîchissement est nécessaire, `false` sinon.
    pub fn check_changed(&mut self) -> Result<bool> {
        // Vérifier l'intervalle de polling
        if self.last_check.elapsed() < CHECK_INTERVAL {
            return Ok(false);
        }

        self.last_check = Instant::now();

        // Stocker les anciennes valeurs
        let old_head = self.head_mtime;
        let old_index = self.index_mtime;
        let old_refs = self.refs_mtime;

        // Mettre à jour les timestamps
        self.update_timestamps()?;

        // Détecter les changements
        let head_changed = self.head_mtime != old_head;
        let index_changed = self.index_mtime != old_index;
        let refs_changed = self.refs_mtime != old_refs;

        if head_changed || index_changed || refs_changed {
            // Enregistrer le moment de la détection
            self.last_change_detected = Some(Instant::now());
        }

        // Vérifier si le debounce est écoulé et qu'un changement a été détecté
        if let Some(change_time) = self.last_change_detected {
            if change_time.elapsed() >= DEBOUNCE_DELAY {
                // Reset pour le prochain changement
                self.last_change_detected = None;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Force une vérification immédiate sans attendre l'intervalle.
    ///
    /// Utile lors d'un rafraîchissement manuel pour réinitialiser
    /// les timestamps de référence.
    pub fn reset(&mut self) -> Result<()> {
        self.last_check = Instant::now();
        self.last_change_detected = None;
        self.update_timestamps()
    }
}

/// Trouve le répertoire `.git/` à partir d'un chemin donné.
///
/// Cherche dans le chemin donné puis dans ses parents.
fn find_git_dir(start_path: &Path) -> Result<PathBuf> {
    let mut current = start_path;

    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            return Ok(git_dir);
        }

        // Remonter vers le parent
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // Si on arrive ici, on n'a pas trouvé de .git/
    // Essayer avec la commande git pour les worktrees
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(start_path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !git_dir.is_empty() {
                return Ok(PathBuf::from(git_dir));
            }
        }
    }

    Err(crate::error::GitSvError::Git(git2::Error::from_str(
        "Répertoire .git/ non trouvé",
    )))
}

/// Récupère le timestamp de dernière modification d'un fichier ou répertoire.
fn get_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        // Créer les fichiers nécessaires
        File::create(git_dir.join("HEAD")).unwrap();
        File::create(git_dir.join("index")).unwrap();
        fs::create_dir(git_dir.join("refs")).unwrap();
        fs::create_dir(git_dir.join("refs/heads")).unwrap();

        let watcher = GitWatcher::new(temp_dir.path());
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_detects_no_changes_initially() {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        File::create(git_dir.join("HEAD")).unwrap();
        File::create(git_dir.join("index")).unwrap();
        fs::create_dir_all(git_dir.join("refs/heads")).unwrap();

        let mut watcher = GitWatcher::new(temp_dir.path()).unwrap();

        // Force check immédiate (pas d'intervalle)
        watcher.last_check = Instant::now() - CHECK_INTERVAL - Duration::from_millis(1);

        // Pas de changement attendu
        assert!(!watcher.check_changed().unwrap());
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
}
