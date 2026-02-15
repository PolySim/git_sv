//! Helpers pour les tests.

use git2::{Repository, Signature};
use std::path::Path;
use tempfile::TempDir;

/// Crée un repository git temporaire pour les tests.
///
/// Retourne le `TempDir` (pour garder le répertoire vivant) et le `Repository`.
pub fn create_test_repo() -> (TempDir, Repository) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo = Repository::init(temp_dir.path()).expect("Failed to init repo");

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

/// Crée un repo avec une histoire linéaire simple.
/// Commits: A -> B -> C
pub fn create_linear_repo() -> (TempDir, Repository) {
    let (temp_dir, repo) = create_test_repo();

    // Commit A
    commit_file(&repo, "file.txt", "content A", "First commit");

    // Commit B
    commit_file(&repo, "file.txt", "content B", "Second commit");

    // Commit C
    commit_file(&repo, "file.txt", "content C", "Third commit");

    (temp_dir, repo)
}

/// Crée un repo avec un merge.
/// Commits: A -> B -> D (merge)
///              -> C ----^
pub fn create_merge_repo() -> (TempDir, Repository) {
    let (temp_dir, repo) = create_test_repo();

    // Commit A (main)
    commit_file(&repo, "file.txt", "content A", "First commit");

    // Commit B (main)
    commit_file(&repo, "file.txt", "content B", "Second commit");

    // Créer une branche feature
    let head = repo
        .head()
        .expect("No HEAD")
        .peel_to_commit()
        .expect("No commit");
    repo.branch("feature", &head, false)
        .expect("Failed to create branch");

    // Commit C (feature)
    let feature_branch = repo
        .find_branch("feature", git2::BranchType::Local)
        .expect("Failed to find branch");
    let feature_ref = feature_branch.get();
    repo.set_head(feature_ref.name().expect("Invalid refname"))
        .expect("Failed to set HEAD");
    commit_file(&repo, "feature.txt", "feature content", "Feature commit");

    // Retourner sur main et merger
    repo.set_head("refs/heads/main")
        .expect("Failed to set HEAD");
    let main_commit = repo
        .head()
        .expect("No HEAD")
        .peel_to_commit()
        .expect("No commit");
    let feature_commit = repo
        .find_branch("feature", git2::BranchType::Local)
        .expect("No feature branch")
        .get()
        .peel_to_commit()
        .expect("No feature commit");

    // Merge
    repo.merge(&[&feature_commit], None, None)
        .expect("Failed to merge");

    // Commit le merge
    if repo.index().expect("No index").has_conflicts() {
        panic!("Merge conflicts!");
    }

    let sig = Signature::now("Test User", "test@example.com").expect("Failed to create signature");
    let mut index = repo.index().expect("Failed to get index");
    let tree_oid = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_oid).expect("Failed to find tree");

    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Merge commit",
        &tree,
        &[&main_commit, &feature_commit],
    )
    .expect("Failed to create merge commit");

    // Nettoyer
    repo.cleanup_state().expect("Failed to cleanup");

    (temp_dir, repo)
}
