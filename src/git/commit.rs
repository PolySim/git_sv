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
        let message = commit
            .summary()
            .unwrap_or("")
            .to_string();
        let author = commit
            .author()
            .name()
            .unwrap_or("Inconnu")
            .to_string();
        let email = commit
            .author()
            .email()
            .unwrap_or("")
            .to_string();
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
    let sig = repo.signature().or_else(|_| {
        Signature::now("git_sv", "git_sv@local")
    })?;

    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent_commit = match repo.head() {
        Ok(head) => {
            let oid = head.target().expect("HEAD devrait pointer vers un commit");
            Some(repo.find_commit(oid)?)
        }
        Err(_) => None, // Premier commit du repo.
    };

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    let oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parents,
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
