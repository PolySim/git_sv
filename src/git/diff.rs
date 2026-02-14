use git2::{Oid, Repository};

use crate::error::Result;

/// Statut d'une modification de fichier.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Information sur un fichier modifié dans un commit.
#[derive(Debug, Clone)]
pub struct DiffFile {
    pub path: String,
    pub status: DiffStatus,
    pub old_path: Option<String>,
    pub additions: usize,
    pub deletions: usize,
}

/// Calcule le diff d'un commit donné.
///
/// Retourne la liste des fichiers modifiés avec leurs stats (+/-).
pub fn commit_diff(repo: &Repository, oid: Oid) -> Result<Vec<DiffFile>> {
    let commit = repo.find_commit(oid)?;
    let commit_tree = commit.tree()?;

    // Obtenir l'arbre du parent (si existe).
    let parent_tree = if commit.parent_count() > 0 {
        let parent = commit.parent(0)?;
        Some(parent.tree()?)
    } else {
        None
    };

    // Calculer le diff.
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;

    let mut files = Vec::new();

    // Itérer sur les deltas avec leur index.
    for (idx, delta) in diff.deltas().enumerate() {
        let status = match delta.status() {
            git2::Delta::Added => DiffStatus::Added,
            git2::Delta::Modified => DiffStatus::Modified,
            git2::Delta::Deleted => DiffStatus::Deleted,
            git2::Delta::Renamed => DiffStatus::Renamed,
            _ => continue, // Ignorer les autres types.
        };

        let path = delta
            .new_file()
            .path()
            .and_then(|p| p.to_str())
            .unwrap_or("???")
            .to_string();

        let old_path = if status == DiffStatus::Renamed {
            delta
                .old_file()
                .path()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        // Calculer les stats de lignes via le patch.
        let (additions, deletions) = if let Ok(Some(patch)) = git2::Patch::from_diff(&diff, idx) {
            count_patch_lines(&patch)
        } else {
            (0, 0)
        };

        files.push(DiffFile {
            path,
            status,
            old_path,
            additions,
            deletions,
        });
    }

    Ok(files)
}

/// Compte les lignes ajoutées et supprimées dans un patch.
fn count_patch_lines(patch: &git2::Patch) -> (usize, usize) {
    let mut additions = 0;
    let mut deletions = 0;

    // Obtenir les stats du patch.
    // line_stats() retourne un tuple (total_lines, additions, deletions)
    if let Ok((_, add, del)) = patch.line_stats() {
        additions = add;
        deletions = del;
    }

    (additions, deletions)
}

impl DiffStatus {
    /// Retourne le caractère d'affichage pour le statut.
    pub fn display_char(&self) -> char {
        match self {
            DiffStatus::Added => 'A',
            DiffStatus::Modified => 'M',
            DiffStatus::Deleted => 'D',
            DiffStatus::Renamed => 'R',
        }
    }
}
