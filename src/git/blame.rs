use crate::error::Result;
use git2::{BlameOptions, Oid, Repository};
use std::time::SystemTime;

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

    // Configurer les options de blame
    let mut blame_opts = BlameOptions::new();
    blame_opts.newest_commit(commit_oid);

    // Générer le blame
    let blame = repo.blame_file(std::path::Path::new(file_path), Some(&mut blame_opts))?;

    // Récupérer le contenu du fichier à ce commit
    let tree = commit.tree()?;
    let tree_entry = tree
        .get_path(std::path::Path::new(file_path))
        .map_err(|_| {
            crate::error::GitSvError::FileNotFound {
                path: file_path.to_string(),
            }
        })?;

    let blob = repo.find_blob(tree_entry.id())?;
    let content = String::from_utf8_lossy(blob.content());
    let file_lines: Vec<&str> = content.lines().collect();

    let mut blame_lines = Vec::new();

    // Parcourir chaque ligne du fichier
    for (line_idx, line_content) in file_lines.iter().enumerate() {
        let line_num = line_idx + 1;

        // Récupérer le hunk de blame pour cette ligne
        if let Some(hunk) = blame.get_line(line_num) {
            let hunk_commit_oid = hunk.final_commit_id();
            let hunk_commit = repo.find_commit(hunk_commit_oid)?;
            let author = hunk_commit.author();

            let short_hash = format!("{:.7}", hunk_commit_oid);

            // Convertir le timestamp git2 en SystemTime
            let timestamp = author.when();
            let time_secs = timestamp.seconds() as u64;
            let system_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(time_secs);

            blame_lines.push(BlameLine {
                line_num,
                content: line_content.to_string(),
                commit_oid: hunk_commit_oid,
                author: author.name().unwrap_or("Unknown").to_string(),
                author_email: author.email().unwrap_or("").to_string(),
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
}
