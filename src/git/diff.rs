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

/// Ligne d'un diff avec son type (ajout, suppression, contexte).
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineType {
    /// Ligne de contexte (inchangée).
    Context,
    /// Ligne ajoutée.
    Addition,
    /// Ligne supprimée.
    Deletion,
    /// En-tête de hunk (ex: @@ -10,5 +10,7 @@).
    HunkHeader,
}

/// Ligne individuelle d'un diff.
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// Type de la ligne.
    pub line_type: DiffLineType,
    /// Contenu textuel de la ligne.
    pub content: String,
    /// Numéro de ligne dans l'ancien fichier (si applicable).
    pub old_lineno: Option<u32>,
    /// Numéro de ligne dans le nouveau fichier (si applicable).
    pub new_lineno: Option<u32>,
}

/// Diff complet d'un fichier dans un commit.
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// Chemin du fichier.
    pub path: String,
    /// Statut (Added, Modified, Deleted, Renamed).
    pub status: DiffStatus,
    /// Lignes du diff.
    pub lines: Vec<DiffLine>,
    /// Nombre total d'ajouts.
    pub additions: usize,
    /// Nombre total de suppressions.
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

        // Calculer les stats de lignes via le patch.
        let (additions, deletions) = if let Ok(Some(patch)) = git2::Patch::from_diff(&diff, idx) {
            count_patch_lines(&patch)
        } else {
            (0, 0)
        };

        files.push(DiffFile {
            path,
            status,
            old_path: None,
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

/// Récupère le diff détaillé d'un fichier spécifique dans un commit.
pub fn get_file_diff(repo: &Repository, oid: Oid, file_path: &str) -> Result<FileDiff> {
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

    // Trouver le delta correspondant au fichier.
    for (idx, delta) in diff.deltas().enumerate() {
        let path = delta
            .new_file()
            .path()
            .and_then(|p| p.to_str())
            .unwrap_or("???");

        if path != file_path {
            continue;
        }

        let status = match delta.status() {
            git2::Delta::Added => DiffStatus::Added,
            git2::Delta::Modified => DiffStatus::Modified,
            git2::Delta::Deleted => DiffStatus::Deleted,
            git2::Delta::Renamed => DiffStatus::Renamed,
            _ => continue,
        };

        // Extraire les lignes du patch.
        let mut lines = Vec::new();
        let mut additions = 0;
        let mut deletions = 0;

        if let Ok(Some(patch)) = git2::Patch::from_diff(&diff, idx) {
            for hunk_idx in 0..patch.num_hunks() {
                let (hunk, _) = patch.hunk(hunk_idx)?;
                // Ajouter le header du hunk.
                lines.push(DiffLine {
                    line_type: DiffLineType::HunkHeader,
                    content: format!(
                        "@@ -{},{} +{},{} @@",
                        hunk.old_start(),
                        hunk.old_lines(),
                        hunk.new_start(),
                        hunk.new_lines()
                    ),
                    old_lineno: None,
                    new_lineno: None,
                });

                let num_lines = patch.num_lines_in_hunk(hunk_idx)?;
                for line_idx in 0..num_lines {
                    let line = patch.line_in_hunk(hunk_idx, line_idx)?;
                    let line_type = match line.origin() {
                        '+' => DiffLineType::Addition,
                        '-' => DiffLineType::Deletion,
                        ' ' => DiffLineType::Context,
                        _ => continue,
                    };

                    match line_type {
                        DiffLineType::Addition => additions += 1,
                        DiffLineType::Deletion => deletions += 1,
                        _ => {}
                    }

                    lines.push(DiffLine {
                        line_type,
                        content: String::from_utf8_lossy(line.content())
                            .trim_end()
                            .to_string(),
                        old_lineno: line.old_lineno(),
                        new_lineno: line.new_lineno(),
                    });
                }
            }
        }

        return Ok(FileDiff {
            path: path.to_string(),
            status,
            lines,
            additions,
            deletions,
        });
    }

    Err(crate::error::GitSvError::Git(git2::Error::from_str(
        "Fichier non trouvé dans le commit",
    )))
}

/// Récupère le diff d'un fichier du working directory (non committé).
pub fn working_dir_file_diff(repo: &Repository, file_path: &str) -> Result<FileDiff> {
    let head = repo.head()?;
    let head_oid = head
        .target()
        .ok_or_else(|| git2::Error::from_str("HEAD ne pointe pas vers un commit"))?;
    let head_commit = repo.find_commit(head_oid)?;
    let head_tree = head_commit.tree()?;

    // Options pour le diff entre HEAD et working directory.
    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file_path);

    let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))?;

    // Trouver le delta correspondant au fichier.
    for (idx, delta) in diff.deltas().enumerate() {
        let path = delta
            .new_file()
            .path()
            .and_then(|p| p.to_str())
            .unwrap_or("???");

        if path != file_path {
            continue;
        }

        let status = match delta.status() {
            git2::Delta::Added => DiffStatus::Added,
            git2::Delta::Modified => DiffStatus::Modified,
            git2::Delta::Deleted => DiffStatus::Deleted,
            git2::Delta::Renamed => DiffStatus::Renamed,
            _ => continue,
        };

        // Extraire les lignes du patch.
        let mut lines = Vec::new();
        let mut additions = 0;
        let mut deletions = 0;

        if let Ok(Some(patch)) = git2::Patch::from_diff(&diff, idx) {
            for hunk_idx in 0..patch.num_hunks() {
                let (hunk, _) = patch.hunk(hunk_idx)?;
                // Ajouter le header du hunk.
                lines.push(DiffLine {
                    line_type: DiffLineType::HunkHeader,
                    content: format!(
                        "@@ -{},{} +{},{} @@",
                        hunk.old_start(),
                        hunk.old_lines(),
                        hunk.new_start(),
                        hunk.new_lines()
                    ),
                    old_lineno: None,
                    new_lineno: None,
                });

                let num_lines = patch.num_lines_in_hunk(hunk_idx)?;
                for line_idx in 0..num_lines {
                    let line = patch.line_in_hunk(hunk_idx, line_idx)?;
                    let line_type = match line.origin() {
                        '+' => DiffLineType::Addition,
                        '-' => DiffLineType::Deletion,
                        ' ' => DiffLineType::Context,
                        _ => continue,
                    };

                    match line_type {
                        DiffLineType::Addition => additions += 1,
                        DiffLineType::Deletion => deletions += 1,
                        _ => {}
                    }

                    lines.push(DiffLine {
                        line_type,
                        content: String::from_utf8_lossy(line.content())
                            .trim_end()
                            .to_string(),
                        old_lineno: line.old_lineno(),
                        new_lineno: line.new_lineno(),
                    });
                }
            }
        }

        return Ok(FileDiff {
            path: path.to_string(),
            status,
            lines,
            additions,
            deletions,
        });
    }

    Err(crate::error::GitSvError::Git(git2::Error::from_str(
        "Fichier non trouvé dans le working directory",
    )))
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
