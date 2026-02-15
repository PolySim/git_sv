use git2::{Oid, Repository, Signature};

use crate::error::Result;

/// Informations essentielles d'un commit.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: Oid,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub parents: Vec<Oid>,
}

impl CommitInfo {
    /// Crée un CommitInfo depuis un git2::Commit.
    pub fn from_git2_commit(commit: &git2::Commit) -> Self {
        let message = commit.summary().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("Inconnu").to_string();
        let email = commit.author().email().unwrap_or("").to_string();
        let timestamp = commit.time().seconds();
        let parents = commit.parent_ids().collect();

        Self {
            oid: commit.id(),
            message,
            author,
            email,
            timestamp,
            parents,
        }
    }

    /// Retourne le hash court (7 caractères).
    pub fn short_hash(&self) -> String {
        self.oid.to_string()[..7].to_string()
    }
}

/// Crée un commit avec le message donné sur l'index courant.
pub fn create_commit(repo: &Repository, message: &str) -> Result<Oid> {
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("git_sv", "git_sv@local"))?;

    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent_commit = match repo.head() {
        Ok(head) => {
            let oid = head.target().ok_or_else(|| {
                crate::error::GitSvError::Other("HEAD ne pointe pas vers un commit".into())
            })?;
            Some(repo.find_commit(oid)?)
        }
        Err(_) => None, // Premier commit du repo.
    };

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

    Ok(oid)
}

/// Amende le dernier commit avec le message donné et l'index courant.
pub fn amend_commit(repo: &Repository, message: &str) -> Result<Oid> {
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("git_sv", "git_sv@local"))?;

    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Récupérer le commit HEAD
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;

    // Amender le commit (remplace HEAD)
    let oid = head_commit.amend(
        Some("HEAD"),  // Mettre à jour HEAD
        None,          // Ne pas changer l'auteur
        None,          // Ne pas changer le committer
        None,          // Ne pas changer l'encodage
        Some(message), // Nouveau message
        Some(&tree),   // Nouveau tree
    )?;

    Ok(oid)
}

/// Stage un fichier dans l'index.
pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_path(std::path::Path::new(path))?;
    index.write()?;
    Ok(())
}

/// Stage tous les fichiers modifiés.
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

/// Unstage un fichier (le retirer de l'index, revenir à HEAD).
pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;

    // Réinitialiser ce fichier dans l'index depuis HEAD.
    repo.reset_default(Some(&head_commit.as_object()), [path])?;
    Ok(())
}

/// Unstage tous les fichiers.
pub fn unstage_all(repo: &Repository) -> Result<()> {
    let head = repo.head()?;
    let obj = head.peel(git2::ObjectType::Commit)?;
    repo.reset(&obj, git2::ResetType::Mixed, None)?;
    Ok(())
}

/// Cherry-pick un commit sur la branche courante.
pub fn cherry_pick(repo: &Repository, commit_oid: Oid) -> Result<()> {
    let commit = repo.find_commit(commit_oid)?;

    // Effectuer le cherry-pick
    repo.cherrypick(&commit, None)?;

    // Vérifier s'il y a des conflits
    let mut index = repo.index()?;
    if index.has_conflicts() {
        return Err(crate::error::GitSvError::Other(
            "Cherry-pick a échoué: conflits détectés".into(),
        ));
    }

    // Si pas de conflits, créer le commit
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("git_sv", "git_sv@local"))?;

    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let head = repo.head()?;
    let parent_commit = head.peel_to_commit()?;

    let message = format!(
        "{}\n\n(cherry picked from commit {})",
        commit.message().unwrap_or(""),
        commit_oid
    );

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent_commit])?;

    // Nettoyer l'état de cherry-pick
    repo.cleanup_state()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;
    use std::path::Path;

    #[test]
    fn test_commit_info_from_git2_commit() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un commit
        let oid = commit_file(&repo, "test.txt", "Hello World", "Test commit message");
        let commit = repo.find_commit(oid).unwrap();

        let info = CommitInfo::from_git2_commit(&commit);

        assert_eq!(info.message, "Test commit message");
        assert_eq!(info.author, "Test User");
        assert_eq!(info.email, "test@example.com");
        assert!(info.timestamp > 0);
        assert!(info.parents.is_empty()); // Premier commit
    }

    #[test]
    fn test_commit_info_short_hash() {
        let (_temp_dir, repo) = create_test_repo();

        let oid = commit_file(&repo, "test.txt", "content", "Test");
        let commit = repo.find_commit(oid).unwrap();
        let info = CommitInfo::from_git2_commit(&commit);

        let short = info.short_hash();
        assert_eq!(short.len(), 7);
        assert!(oid.to_string().starts_with(&short));
    }

    #[test]
    fn test_stage_file() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un fichier
        create_file(&repo, "test.txt", "Hello");

        // Stage le fichier
        stage_file(&repo, "test.txt").unwrap();

        // Vérifier qu'il est dans l'index
        let index = repo.index().unwrap();
        let entries: Vec<_> = index.iter().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_stage_all() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer plusieurs fichiers
        create_file(&repo, "file1.txt", "content1");
        create_file(&repo, "file2.txt", "content2");

        // Stage tous les fichiers
        stage_all(&repo).unwrap();

        // Vérifier qu'ils sont dans l'index
        let index = repo.index().unwrap();
        let entries: Vec<_> = index.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_unstage_file() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer et commiter un fichier
        commit_file(&repo, "test.txt", "Initial content", "Initial commit");

        // Modifier le fichier et stage
        create_file(&repo, "test.txt", "Modified content");
        stage_file(&repo, "test.txt").unwrap();

        // Vérifier qu'il est staged (modification présente dans l'index)
        let index = repo.index().unwrap();
        assert_eq!(index.iter().count(), 1);

        // Unstage le fichier - retour à HEAD
        unstage_file(&repo, "test.txt").unwrap();

        // Après unstage, le fichier devrait correspondre à HEAD
        // Donc il n'y a plus de diff entre index et HEAD
        let statuses = repo.statuses(None).unwrap();
        // Le fichier ne devrait plus être dans l'état "staged modified"
        let mut found_staged = false;
        for entry in statuses.iter() {
            if entry.path() == Some("test.txt") {
                let status = entry.status();
                if status.contains(git2::Status::INDEX_MODIFIED) {
                    found_staged = true;
                }
            }
        }
        assert!(!found_staged, "Le fichier ne devrait plus être staged");
    }

    #[test]
    fn test_create_commit() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un fichier et l'ajouter à l'index
        create_file(&repo, "test.txt", "content");
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        // Créer un commit
        let oid = create_commit(&repo, "My commit message").unwrap();

        // Vérifier que le commit existe
        let commit = repo.find_commit(oid).unwrap();
        assert_eq!(commit.summary().unwrap(), "My commit message");
    }
}
