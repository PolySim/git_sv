use git2::{Oid, Repository};

use crate::error::Result;

/// Mode d'affichage du diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffViewMode {
    /// Mode unifié (lignes de contexte + ajouts + suppressions).
    #[default]
    Unified,
    /// Mode côte à côte (ancien vs nouveau).
    SideBySide,
}

impl DiffViewMode {
    /// Bascule entre les modes.
    pub fn toggle(&mut self) {
        *self = match self {
            DiffViewMode::Unified => DiffViewMode::SideBySide,
            DiffViewMode::SideBySide => DiffViewMode::Unified,
        };
    }
}

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

/// Extrait les lignes d'un patch pour un fichier donné.
///
/// Cette fonction factorise la logique commune entre get_file_diff() et working_dir_file_diff().
fn extract_diff_lines(patch: &git2::Patch) -> (Vec<DiffLine>, usize, usize) {
    let mut lines = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;

    for hunk_idx in 0..patch.num_hunks() {
        let Ok((hunk, _)) = patch.hunk(hunk_idx) else {
            continue;
        };

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

        let num_lines = match patch.num_lines_in_hunk(hunk_idx) {
            Ok(n) => n,
            Err(_) => continue,
        };

        for line_idx in 0..num_lines {
            let line = match patch.line_in_hunk(hunk_idx, line_idx) {
                Ok(l) => l,
                Err(_) => continue,
            };

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

    (lines, additions, deletions)
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
    find_and_extract_file_diff(&diff, file_path, "Fichier non trouvé dans le commit")
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
    find_and_extract_file_diff(
        &diff,
        file_path,
        "Fichier non trouvé dans le working directory",
    )
}

/// Trouve un fichier dans un diff et extrait son contenu.
///
/// Cette fonction factorise la logique de recherche et d'extraction
/// utilisée par get_file_diff() et working_dir_file_diff().
fn find_and_extract_file_diff(
    diff: &git2::Diff,
    file_path: &str,
    error_msg: &str,
) -> Result<FileDiff> {
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

        // Extraire les lignes du patch en utilisant la fonction factorisée.
        let (lines, additions, deletions) =
            if let Ok(Some(patch)) = git2::Patch::from_diff(diff, idx) {
                extract_diff_lines(&patch)
            } else {
                (Vec::new(), 0, 0)
            };

        return Ok(FileDiff {
            path: path.to_string(),
            status,
            lines,
            additions,
            deletions,
        });
    }

    Err(crate::error::GitSvError::Git(git2::Error::from_str(
        error_msg,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;
    use std::path::Path;

    #[test]
    fn test_diff_status_display_char() {
        assert_eq!(DiffStatus::Added.display_char(), 'A');
        assert_eq!(DiffStatus::Modified.display_char(), 'M');
        assert_eq!(DiffStatus::Deleted.display_char(), 'D');
        assert_eq!(DiffStatus::Renamed.display_char(), 'R');
    }

    #[test]
    fn test_commit_diff_simple() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        let oid = commit_file(&repo, "test.txt", "Hello World", "Initial commit");

        // Obtenir le diff du commit
        let files = commit_diff(&repo, oid).unwrap();

        // Devrait avoir 1 fichier ajouté
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert!(matches!(files[0].status, DiffStatus::Added));
        assert!(files[0].additions > 0);
    }

    #[test]
    fn test_commit_diff_multiple_files() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit avec plusieurs fichiers
        create_file(&repo, "file1.txt", "content1");
        create_file(&repo, "file2.txt", "content2");
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file1.txt")).unwrap();
        index.add_path(Path::new("file2.txt")).unwrap();
        index.write().unwrap();
        let oid = commit(&repo, "Multi-file commit");

        // Obtenir le diff
        let files = commit_diff(&repo, oid).unwrap();

        // Devrait avoir 2 fichiers
        assert_eq!(files.len(), 2);
        let paths: Vec<_> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"file1.txt"));
        assert!(paths.contains(&"file2.txt"));
    }

    #[test]
    fn test_commit_diff_modification() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "Initial content", "Initial commit");

        // Modifier le fichier et committer
        let oid = commit_file(&repo, "test.txt", "Modified content", "Second commit");

        // Obtenir le diff
        let files = commit_diff(&repo, oid).unwrap();

        // Devrait avoir 1 fichier modifié
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert!(matches!(files[0].status, DiffStatus::Modified));
    }

    #[test]
    fn test_get_file_diff() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial avec un fichier
        let _oid = commit_file(
            &repo,
            "test.txt",
            "Line 1\nLine 2\nLine 3\n",
            "Initial commit",
        );

        // Modifier le fichier
        let oid2 = commit_file(
            &repo,
            "test.txt",
            "Line 1\nModified Line 2\nLine 3\n",
            "Second commit",
        );

        // Obtenir le diff détaillé
        let file_diff = get_file_diff(&repo, oid2, "test.txt").unwrap();

        assert_eq!(file_diff.path, "test.txt");
        assert!(matches!(file_diff.status, DiffStatus::Modified));
        // Devrait avoir au moins quelques lignes
        assert!(!file_diff.lines.is_empty());
    }

    #[test]
    fn test_working_dir_file_diff() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "Initial content", "Initial commit");

        // Modifier le fichier sans committer
        create_file(&repo, "test.txt", "Modified in working dir");

        // Obtenir le diff du working directory
        let file_diff = working_dir_file_diff(&repo, "test.txt").unwrap();

        assert_eq!(file_diff.path, "test.txt");
        assert!(matches!(file_diff.status, DiffStatus::Modified));
        assert!(!file_diff.lines.is_empty());
    }
}
