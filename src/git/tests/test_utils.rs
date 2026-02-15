//! Utilitaires pour les tests unitaires du module git.

use git2::{Repository, Signature};
use std::path::Path;
use tempfile::TempDir;

/// Crée un repository git temporaire pour les tests.
///
/// Retourne le `TempDir` (pour garder le répertoire vivant) et le `Repository`.
pub fn create_test_repo() -> (TempDir, Repository) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head("main");
    let repo = Repository::init_opts(temp_dir.path(), &opts).expect("Failed to init repo");

    // Configurer git pour les commits
    let mut config = repo.config().expect("Failed to get config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set user.name");
    config
        .set_str("user.email", "test@example.com")
        .expect("Failed to set user.email");

    (temp_dir, repo)
}

/// Crée un fichier et l'ajoute à l'index.
pub fn create_file(repo: &Repository, path: &str, content: &str) {
    let workdir = repo.workdir().expect("No workdir");
    let full_path = workdir.join(path);

    // Créer les répertoires parents si nécessaire
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create directories");
    }

    std::fs::write(&full_path, content).expect("Failed to write file");
}

/// Commit les changements de l'index avec un message.
pub fn commit(repo: &Repository, message: &str) -> git2::Oid {
    let sig = Signature::now("Test User", "test@example.com").expect("Failed to create signature");
    let mut index = repo.index().expect("Failed to get index");
    let tree_oid = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_oid).expect("Failed to find tree");

    let parent_commit = repo
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repo.find_commit(oid).ok());

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .expect("Failed to create commit")
}

/// Crée un commit avec un fichier donné.
pub fn commit_file(repo: &Repository, path: &str, content: &str, message: &str) -> git2::Oid {
    create_file(repo, path, content);
    let mut index = repo.index().expect("Failed to get index");
    index.add_path(Path::new(path)).expect("Failed to add file");
    index.write().expect("Failed to write index");
    commit(repo, message)
}
