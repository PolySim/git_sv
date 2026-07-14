//! Parsing et représentation des diffs de fichiers.
//!
//! Supporte le mode unifié et le mode side-by-side.

use git2::{Oid, Repository};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::error::Result;

/// Nombre maximum de lignes de diff matérialisées en mémoire.
const MAX_DIFF_LINES: usize = 20_000;
/// Taille maximum affichée pour un fichier non suivi.
const MAX_UNTRACKED_DIFF_BYTES: u64 = 1_048_576;
/// Taille maximale d'une image conservée pour la prévisualisation.
const MAX_IMAGE_PREVIEW_BYTES: usize = 20 * 1_048_576;

/// Format d'une image prévisualisable dans le terminal.
#[cfg_attr(not(feature = "image-preview"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Raster,
    Svg,
}

/// Contenu brut d'une image, partagé entre le diff et son cache LRU.
#[cfg_attr(not(feature = "image-preview"), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct ImagePreview {
    pub bytes: Arc<[u8]>,
    pub format: ImageFormat,
}

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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    /// Statut (Added, Modified, Deleted, Renamed).
    pub status: DiffStatus,
    /// Lignes du diff.
    pub lines: Vec<DiffLine>,
    /// Nombre total d'ajouts.
    pub additions: usize,
    /// Nombre total de suppressions.
    pub deletions: usize,
    /// Image correspondant au nouvel état du fichier, si prise en charge.
    pub image_preview: Option<ImagePreview>,
}

impl FileDiff {
    /// Estime la mémoire allouée sur le tas par ce diff et son image éventuelle.
    pub fn estimated_memory_bytes(&self) -> usize {
        let lines = self.lines.capacity() * std::mem::size_of::<DiffLine>()
            + self
                .lines
                .iter()
                .map(|line| line.content.capacity())
                .sum::<usize>();
        let image = self
            .image_preview
            .as_ref()
            .map_or(0, |preview| preview.bytes.len());

        std::mem::size_of::<Self>() + self.path.capacity() + lines + image
    }
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
    let mut diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)?;
    diff.find_similar(None)?;

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

        let (path, old_path) = diff_paths(&delta);

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

/// Extrait les lignes d'un patch pour un fichier donné.
///
/// Cette fonction factorise la logique commune entre get_file_diff() et working_dir_file_diff().
fn extract_diff_lines(patch: &git2::Patch) -> (Vec<DiffLine>, usize, usize) {
    let mut lines = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;
    let mut truncated = false;

    for hunk_idx in 0..patch.num_hunks() {
        if lines.len() >= MAX_DIFF_LINES {
            truncated = true;
            break;
        }

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

            if lines.len() >= MAX_DIFF_LINES {
                truncated = true;
                break;
            }

            lines.push(DiffLine {
                line_type,
                content: String::from_utf8_lossy(line.content())
                    .trim_end()
                    .replace('\t', "    ")
                    .to_string(),
                old_lineno: line.old_lineno(),
                new_lineno: line.new_lineno(),
            });
        }

        if truncated {
            break;
        }
    }

    if truncated {
        lines.push(limit_message_line(format!(
            "Diff tronque apres {} lignes pour proteger la memoire",
            MAX_DIFF_LINES
        )));
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
    let mut file_diff =
        find_and_extract_file_diff(&diff, file_path, "Fichier non trouvé dans le commit")?;
    let source_tree = if file_diff.status == DiffStatus::Deleted {
        parent_tree.as_ref()
    } else {
        Some(&commit_tree)
    };
    file_diff.image_preview =
        source_tree.and_then(|tree| tree_image_preview(repo, tree, file_path));
    Ok(file_diff)
}

/// Récupère le diff d'un fichier du working directory (non committé).
pub fn working_dir_file_diff(repo: &Repository, file_path: &str) -> Result<FileDiff> {
    let status = repo.status_file(std::path::Path::new(file_path))?;
    if status.is_wt_new() && !status.is_index_new() {
        return build_untracked_file_diff(repo, file_path);
    }

    let head = repo.head()?;
    let head_oid = head
        .target()
        .ok_or_else(|| git2::Error::from_str("HEAD ne pointe pas vers un commit"))?;
    let head_commit = repo.find_commit(head_oid)?;
    let head_tree = head_commit.tree()?;

    let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), None)?;

    // Trouver le delta correspondant au fichier.
    let mut file_diff = find_and_extract_file_diff(
        &diff,
        file_path,
        "Fichier non trouvé dans le working directory",
    )?;
    file_diff.image_preview = working_tree_image_preview(repo, &head_tree, file_path);
    Ok(file_diff)
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
        let (path, old_path) = diff_paths(&delta);
        let matches_requested_path = path == file_path || (old_path.as_deref() == Some(file_path));

        if !matches_requested_path {
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
                (
                    vec![limit_message_line(
                        "Diff non affichable (fichier binaire ou patch indisponible)",
                    )],
                    0,
                    0,
                )
            };

        return Ok(FileDiff {
            path,
            status,
            lines,
            additions,
            deletions,
            image_preview: None,
        });
    }

    Err(crate::error::GitSvError::Git(git2::Error::from_str(
        error_msg,
    )))
}

fn build_untracked_file_diff(repo: &Repository, file_path: &str) -> Result<FileDiff> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Impossible de trouver le chemin du repository"))?;
    let full_path = workdir.join(file_path);
    let metadata = fs::metadata(&full_path)?;

    let format = image_format(file_path);
    let max_size = if format.is_some() {
        MAX_IMAGE_PREVIEW_BYTES as u64
    } else {
        MAX_UNTRACKED_DIFF_BYTES
    };
    if metadata.len() > max_size {
        return Ok(limited_file_diff(
            file_path,
            DiffStatus::Added,
            format!(
                "Fichier non suivi trop volumineux pour prévisualisation ({} octets)",
                metadata.len()
            ),
        ));
    }

    let bytes = fs::read(&full_path)?;
    if let Some(format) = format {
        return Ok(FileDiff {
            path: file_path.to_string(),
            status: DiffStatus::Added,
            lines: Vec::new(),
            additions: 0,
            deletions: 0,
            image_preview: build_image_preview(format, bytes),
        });
    }
    if bytes.contains(&0) {
        return Ok(limited_file_diff(
            file_path,
            DiffStatus::Added,
            "Fichier binaire non suivi non affichable",
        ));
    }

    let content = match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(_) => {
            return Ok(limited_file_diff(
                file_path,
                DiffStatus::Added,
                "Fichier non suivi non UTF-8 non affichable",
            ));
        }
    };

    let lines: Vec<DiffLine> = content
        .lines()
        .take(MAX_DIFF_LINES)
        .enumerate()
        .map(|(index, line)| DiffLine {
            line_type: DiffLineType::Addition,
            content: line.replace('\t', "    "),
            old_lineno: None,
            new_lineno: Some((index + 1) as u32),
        })
        .collect();
    let truncated = content.lines().count() > MAX_DIFF_LINES;
    let additions = lines.len();
    let mut lines = lines;

    if truncated {
        lines.push(limit_message_line(format!(
            "Diff tronque apres {} lignes pour proteger la memoire",
            MAX_DIFF_LINES
        )));
    }

    Ok(FileDiff {
        path: file_path.to_string(),
        status: DiffStatus::Added,
        additions,
        deletions: 0,
        lines,
        image_preview: None,
    })
}

fn limited_file_diff(file_path: &str, status: DiffStatus, message: impl Into<String>) -> FileDiff {
    FileDiff {
        path: file_path.to_string(),
        status,
        lines: vec![limit_message_line(message)],
        additions: 0,
        deletions: 0,
        image_preview: None,
    }
}

fn build_image_preview(format: ImageFormat, bytes: Vec<u8>) -> Option<ImagePreview> {
    if bytes.len() > MAX_IMAGE_PREVIEW_BYTES {
        return None;
    }

    Some(ImagePreview {
        bytes: Arc::from(bytes),
        format,
    })
}

#[cfg(feature = "image-preview")]
fn image_format(file_path: &str) -> Option<ImageFormat> {
    let extension = Path::new(file_path)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();
    Some(match extension.as_str() {
        "svg" => ImageFormat::Svg,
        "gif" | "jpg" | "jpeg" | "png" | "webp" => ImageFormat::Raster,
        _ => return None,
    })
}

#[cfg(not(feature = "image-preview"))]
fn image_format(_file_path: &str) -> Option<ImageFormat> {
    None
}

fn tree_image_preview(
    repo: &Repository,
    tree: &git2::Tree<'_>,
    file_path: &str,
) -> Option<ImagePreview> {
    let format = image_format(file_path)?;
    let entry = tree.get_path(Path::new(file_path)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    if blob.size() > MAX_IMAGE_PREVIEW_BYTES {
        return None;
    }
    build_image_preview(format, blob.content().to_vec())
}

fn working_tree_image_preview(
    repo: &Repository,
    head_tree: &git2::Tree<'_>,
    file_path: &str,
) -> Option<ImagePreview> {
    let format = image_format(file_path)?;
    if let Some(full_path) = repo.workdir().map(|workdir| workdir.join(file_path)) {
        if let Ok(metadata) = fs::metadata(&full_path) {
            if metadata.len() > MAX_IMAGE_PREVIEW_BYTES as u64 {
                return None;
            }
            return fs::read(full_path)
                .ok()
                .and_then(|bytes| build_image_preview(format, bytes));
        }
    }

    tree_image_preview(repo, head_tree, file_path)
}

fn limit_message_line(message: impl Into<String>) -> DiffLine {
    DiffLine {
        line_type: DiffLineType::HunkHeader,
        content: message.into(),
        old_lineno: None,
        new_lineno: None,
    }
}

fn diff_paths(delta: &git2::DiffDelta<'_>) -> (String, Option<String>) {
    let new_path = delta
        .new_file()
        .path()
        .and_then(|p| p.to_str())
        .map(str::to_string);
    let old_path = delta
        .old_file()
        .path()
        .and_then(|p| p.to_str())
        .map(str::to_string);

    let display_path = new_path
        .clone()
        .or_else(|| old_path.clone())
        .unwrap_or_else(|| "???".to_string());

    let previous_path = match (&old_path, &new_path) {
        (Some(old), Some(new)) if old != new => Some(old.clone()),
        _ => None,
    };

    (display_path, previous_path)
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

    fn delete_file(repo: &Repository, path: &str) {
        let workdir = repo.workdir().unwrap();
        std::fs::remove_file(workdir.join(path)).unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new(path)).unwrap();
        index.write().unwrap();
    }

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
    fn test_get_file_diff_exposes_svg_preview() {
        let (_temp_dir, repo) = create_test_repo();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"><rect width="12" height="8"/></svg>"#;
        let oid = commit_file(&repo, "logo.svg", svg, "Add logo");

        let file_diff = get_file_diff(&repo, oid, "logo.svg").unwrap();
        let preview = file_diff.image_preview.expect("prévisualisation SVG");

        assert_eq!(preview.format, ImageFormat::Svg);
        assert_eq!(preview.bytes.as_ref(), svg.as_bytes());
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

    #[test]
    fn test_untracked_png_exposes_image_preview() {
        let (_temp_dir, repo) = create_test_repo();
        commit_file(&repo, "tracked.txt", "content", "Initial commit");
        let png = b"\x89PNG\r\n\x1a\npreview";
        std::fs::write(repo.workdir().unwrap().join("preview.png"), png).unwrap();

        let file_diff = working_dir_file_diff(&repo, "preview.png").unwrap();
        let preview = file_diff.image_preview.expect("prévisualisation PNG");

        assert_eq!(preview.format, ImageFormat::Raster);
        assert_eq!(preview.bytes.as_ref(), png);
    }

    #[test]
    fn test_image_preview_rejects_oversized_buffer() {
        let preview =
            build_image_preview(ImageFormat::Raster, vec![0; MAX_IMAGE_PREVIEW_BYTES + 1]);

        assert!(preview.is_none());
    }

    #[test]
    fn test_commit_diff_deleted_file_uses_deleted_path() {
        let (_temp_dir, repo) = create_test_repo();

        commit_file(&repo, "docs/test.txt", "Hello", "Initial commit");
        delete_file(&repo, "docs/test.txt");
        let oid = commit(&repo, "Delete file");

        let files = commit_diff(&repo, oid).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "docs/test.txt");
        assert!(matches!(files[0].status, DiffStatus::Deleted));
    }

    #[test]
    fn test_working_dir_file_diff_deleted_file() {
        let (_temp_dir, repo) = create_test_repo();

        commit_file(&repo, "docs/test.txt", "Hello", "Initial commit");
        let workdir = repo.workdir().unwrap();
        std::fs::remove_file(workdir.join("docs/test.txt")).unwrap();

        let file_diff = working_dir_file_diff(&repo, "docs/test.txt").unwrap();

        assert_eq!(file_diff.path, "docs/test.txt");
        assert!(matches!(file_diff.status, DiffStatus::Deleted));
    }

    #[test]
    fn test_working_dir_file_diff_untracked_shows_file_content() {
        let (temp_dir, repo) = create_test_repo();

        commit_file(&repo, "tracked.txt", "tracked\n", "Initial commit");
        std::fs::write(temp_dir.path().join("untracked.txt"), "line 1\nline 2\n").unwrap();

        let file_diff = working_dir_file_diff(&repo, "untracked.txt").unwrap();

        assert_eq!(file_diff.path, "untracked.txt");
        assert!(matches!(file_diff.status, DiffStatus::Added));
        assert_eq!(file_diff.additions, 2);
        assert_eq!(file_diff.deletions, 0);
        assert_eq!(file_diff.lines.len(), 2);
        assert_eq!(file_diff.lines[0].line_type, DiffLineType::Addition);
        assert_eq!(file_diff.lines[0].content, "line 1");
        assert_eq!(file_diff.lines[0].new_lineno, Some(1));
        assert_eq!(file_diff.lines[1].content, "line 2");
    }

    #[test]
    fn test_working_dir_file_diff_untracked_binary_is_limited() {
        let (temp_dir, repo) = create_test_repo();

        commit_file(&repo, "tracked.txt", "tracked\n", "Initial commit");
        std::fs::write(temp_dir.path().join("binary.bin"), [0, 1, 2, 3]).unwrap();

        let file_diff = working_dir_file_diff(&repo, "binary.bin").unwrap();

        assert_eq!(file_diff.path, "binary.bin");
        assert!(matches!(file_diff.status, DiffStatus::Added));
        assert_eq!(file_diff.additions, 0);
        assert_eq!(file_diff.lines.len(), 1);
        assert!(file_diff.lines[0].content.contains("binaire"));
    }
}
