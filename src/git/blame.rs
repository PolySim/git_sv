//! Blame git : annotations de commit par ligne de fichier.

use std::time::{Duration, SystemTime};

use git2::{BlameOptions, Oid, Repository};

use crate::error::Result;

/// Bornes appliquées avant le calcul natif et pendant la matérialisation du blame.
const MAX_BLAME_BYTES: usize = 4 * 1_048_576;
const MAX_BLAME_LINES: usize = 100_000;
const MAX_BLAME_MEMORY_BYTES: usize = 32 * 1_048_576;

/// Information sur une ligne d'un fichier avec son auteur et commit.
#[derive(Debug, Clone)]
pub struct BlameLine {
    /// Numéro de ligne (1-indexed).
    pub line_num: usize,
    /// Contenu de la ligne.
    pub content: String,
    /// Hash du commit qui a introduit cette ligne.
    pub commit_oid: Oid,
    /// Nom de l'auteur.
    pub author: String,
    /// Email de l'auteur.
    #[cfg_attr(not(test), allow(dead_code))]
    pub author_email: String,
    /// Timestamp du commit.
    pub timestamp: SystemTime,
    /// Hash court du commit (7 premiers caractères).
    pub short_hash: String,
}

/// Résultat complet du blame pour un fichier.
#[derive(Debug, Clone)]
pub struct FileBlame {
    /// Chemin du fichier.
    pub path: String,
    /// Lignes annotées.
    pub lines: Vec<BlameLine>,
}

/// Génère le blame pour un fichier à un commit donné.
pub fn blame_file(repo: &Repository, commit_oid: Oid, file_path: &str) -> Result<FileBlame> {
    // Récupérer le commit
    let commit = repo.find_commit(commit_oid)?;

    // Récupérer le contenu du fichier à ce commit
    let tree = commit.tree()?;
    let tree_entry = tree
        .get_path(std::path::Path::new(file_path))
        .map_err(|_| crate::error::GitSvError::FileNotFound {
            path: file_path.to_string(),
        })?;

    if repo.odb()?.read_header(tree_entry.id())?.0 > MAX_BLAME_BYTES {
        return Err(crate::error::GitSvError::Other(
            "Fichier trop volumineux pour le blame (limite : 4 MiB)".to_string(),
        ));
    }
    let blob = repo.find_blob(tree_entry.id())?;
    let content = String::from_utf8_lossy(blob.content());
    if content.lines().take(MAX_BLAME_LINES + 1).count() > MAX_BLAME_LINES {
        return Err(crate::error::GitSvError::Other(
            "Fichier trop long pour le blame (limite : 100 000 lignes)".to_string(),
        ));
    }

    let mut blame_opts = BlameOptions::new();
    blame_opts.newest_commit(commit_oid);
    let blame = repo.blame_file(std::path::Path::new(file_path), Some(&mut blame_opts))?;

    let mut blame_lines = Vec::new();
    let mut memory_bytes: usize = 0;

    // Parcourir chaque ligne du fichier
    for (line_idx, line_content) in content.lines().enumerate() {
        let line_num = line_idx + 1;

        // Récupérer le hunk de blame pour cette ligne
        if let Some(hunk) = blame.get_line(line_num) {
            let hunk_commit_oid = hunk.final_commit_id();
            // La signature est déjà disponible dans le hunk, sans relire le commit par ligne.
            let author = hunk.final_signature().ok_or_else(|| {
                crate::error::GitSvError::Other("Signature absente dans le blame".to_string())
            })?;
            let author_name = author.name().unwrap_or("Unknown");
            let author_email = author.email().unwrap_or("");

            memory_bytes = memory_bytes.saturating_add(
                std::mem::size_of::<BlameLine>() * 2
                    + line_content.len()
                    + author_name.len()
                    + author_email.len()
                    + 7,
            );
            if memory_bytes > MAX_BLAME_MEMORY_BYTES {
                return Err(crate::error::GitSvError::Other(
                    "Blame trop volumineux pour être affiché (limite : 32 MiB)".to_string(),
                ));
            }

            let short_hash = format!("{:.7}", hunk_commit_oid);

            // Convertir le timestamp git2 en SystemTime
            let timestamp = author.when();
            let seconds = timestamp.seconds();
            let duration = Duration::from_secs(seconds.unsigned_abs());
            let system_time = if seconds < 0 {
                SystemTime::UNIX_EPOCH.checked_sub(duration)
            } else {
                SystemTime::UNIX_EPOCH.checked_add(duration)
            }
            .ok_or_else(|| {
                crate::error::GitSvError::Other("Date de commit hors limites".to_string())
            })?;

            blame_lines.push(BlameLine {
                line_num,
                content: line_content.to_string(),
                commit_oid: hunk_commit_oid,
                author: author_name.to_string(),
                author_email: author_email.to_string(),
                timestamp: system_time,
                short_hash,
            });
        }
    }

    Ok(FileBlame {
        path: file_path.to_string(),
        lines: blame_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::create_test_repo;

    #[test]
    fn test_blame_file() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un fichier de test avec plusieurs commits
        let file_path = "test.txt";
        let repo_path = repo.path().parent().unwrap();
        let file_full_path: std::path::PathBuf = repo_path.join(file_path);

        // Premier commit
        std::fs::write(&file_full_path, "line 1\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(file_path)).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let commit1 = repo
            .commit(Some("HEAD"), &sig, &sig, "First line", &tree, &[])
            .unwrap();

        // Deuxième commit - ajouter une ligne
        std::fs::write(&file_full_path, "line 1\nline 2\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(file_path)).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.find_commit(commit1).unwrap();
        let commit2 = repo
            .commit(Some("HEAD"), &sig, &sig, "Second line", &tree, &[&parent])
            .unwrap();

        // Tester le blame
        let blame = blame_file(&repo, commit2, file_path).unwrap();

        assert_eq!(blame.path, file_path);
        assert_eq!(blame.lines.len(), 2);
        assert_eq!(blame.lines[0].content, "line 1");
        assert_eq!(blame.lines[1].content, "line 2");
        assert_eq!(blame.lines[0].commit_oid, commit1);
        assert_eq!(blame.lines[1].commit_oid, commit2);
        assert_eq!(blame.lines[0].author, "Test");
        assert_eq!(blame.lines[0].author_email, "test@example.com");
    }

    #[test]
    fn audit_blame_supports_dates_before_unix_epoch() {
        let (_temp_dir, repo) = create_test_repo();
        std::fs::write(repo.workdir().unwrap().join("old.txt"), "old line\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("old.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let commit = format!("tree {tree_id}\nauthor Past <past@example.com> -1 +0000\ncommitter Past <past@example.com> -1 +0000\n\nOld commit\n");
        let oid = repo
            .odb()
            .unwrap()
            .write(git2::ObjectType::Commit, commit.as_bytes())
            .unwrap();

        let blame = blame_file(&repo, oid, "old.txt").unwrap();

        assert_eq!(
            blame.lines[0].timestamp,
            SystemTime::UNIX_EPOCH - std::time::Duration::from_secs(1)
        );
    }

    #[test]
    fn audit_blame_rejects_large_files_before_annotation() {
        let (_temp_dir, repo) = create_test_repo();
        let oid = crate::git::tests::test_utils::commit_file(
            &repo,
            "large.txt",
            &"x".repeat(MAX_BLAME_BYTES + 1),
            "large",
        );

        assert!(blame_file(&repo, oid, "large.txt")
            .unwrap_err()
            .to_string()
            .contains("4 MiB"));
    }

    #[test]
    fn audit_blame_rejects_excessive_line_counts() {
        let (_temp_dir, repo) = create_test_repo();
        let oid = crate::git::tests::test_utils::commit_file(
            &repo,
            "many.txt",
            &"x\n".repeat(MAX_BLAME_LINES + 1),
            "many lines",
        );

        assert!(blame_file(&repo, oid, "many.txt")
            .unwrap_err()
            .to_string()
            .contains("100 000"));
    }
}
