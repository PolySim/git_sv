//! Suppression des modifications non committées (discard).

use git2::Repository;
use std::fs;

use crate::error::Result;

/// Discard les modifications d'un fichier spécifique (git checkout -- file).
/// Cette opération restaure le fichier à son état dans l'index, sans modifier le staging.
pub fn discard_file(repo: &Repository, file_path: &str) -> Result<()> {
    let status = repo.status_file(std::path::Path::new(file_path))?;

    if status.is_wt_new() && !status.is_index_new() {
        let workdir = repo.workdir().ok_or_else(|| {
            git2::Error::from_str("Impossible de trouver le chemin du repository")
        })?;
        let full_path = workdir.join(file_path);

        let metadata = fs::symlink_metadata(&full_path)?;
        if metadata.is_dir() {
            fs::remove_dir_all(&full_path)?;
        } else {
            fs::remove_file(&full_path)?;
        }

        return Ok(());
    }

    // Restaurer un chemin littéral, même pour un nom contenant des crochets.
    let workdir = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Working tree introuvable"))?;
    let output = std::process::Command::new("git")
        .args([
            "--literal-pathspecs",
            "restore",
            "--worktree",
            "--",
            file_path,
        ])
        .current_dir(workdir)
        .output()?;
    if !output.status.success() {
        return Err(crate::error::GitSvError::OperationFailed {
            operation: "git restore",
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(())
}

/// Discard toutes les modifications non stagées (git checkout -- .).
/// Cette opération restaure les fichiers depuis l'index et conserve le staging.
pub fn discard_all(repo: &Repository) -> Result<()> {
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    checkout_builder.update_index(false);
    checkout_builder.remove_untracked(true);

    repo.checkout_index(None, Some(&mut checkout_builder))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn discard_preserves_staged_content_and_index() {
        for discard_everything in [false, true] {
            let (directory, repo) = create_test_repo();
            commit_file(&repo, "file.txt", "committed\n", "initial");
            let path = directory.path().join("file.txt");
            fs::write(&path, "staged\n").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let staged_tree = index.write_tree().unwrap();
            fs::write(&path, "unstaged\n").unwrap();

            if discard_everything {
                discard_all(&repo).unwrap();
            } else {
                discard_file(&repo, "file.txt").unwrap();
            }

            assert_eq!(fs::read_to_string(&path).unwrap(), "staged\n");
            assert_eq!(repo.index().unwrap().write_tree().unwrap(), staged_tree);
        }
    }

    #[test]
    fn discard_new_file_preserves_staging_before_first_commit() {
        let (directory, repo) = create_test_repo();
        let path = directory.path().join("new.txt");
        fs::write(&path, "staged\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("new.txt")).unwrap();
        index.write().unwrap();
        fs::write(&path, "unstaged\n").unwrap();

        discard_file(&repo, "new.txt").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "staged\n");
        assert!(repo
            .status_file(std::path::Path::new("new.txt"))
            .unwrap()
            .is_index_new());
    }

    #[test]
    fn discard_file_uses_literal_path() {
        let (directory, repo) = create_test_repo();
        commit_file(&repo, "[a].txt", "selected base\n", "initial");
        commit_file(&repo, "a.txt", "other base\n", "other");
        fs::write(directory.path().join("[a].txt"), "selected change\n").unwrap();
        fs::write(directory.path().join("a.txt"), "other change\n").unwrap();
        discard_file(&repo, "[a].txt").unwrap();
        assert_eq!(
            fs::read_to_string(directory.path().join("[a].txt")).unwrap(),
            "selected base\n"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("a.txt")).unwrap(),
            "other change\n"
        );
    }

    #[test]
    fn test_discard_file() {
        let (temp_dir, repo) = create_test_repo();
        commit_file(&repo, "test.txt", "initial content\n", "Initial commit");

        // Modifier le fichier
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "modified content\n").unwrap();

        // Vérifier que le fichier est modifié
        let content_before = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content_before, "modified content\n");

        // Discard les modifications
        discard_file(&repo, "test.txt").unwrap();

        // Vérifier que le fichier est restauré
        let content_after = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content_after, "initial content\n");
    }

    #[test]
    fn test_discard_all() {
        let (temp_dir, repo) = create_test_repo();
        commit_file(&repo, "file1.txt", "content1\n", "Add file1");
        commit_file(&repo, "file2.txt", "content2\n", "Add file2");

        // Modifier les deux fichiers
        let file1_path = temp_dir.path().join("file1.txt");
        let file2_path = temp_dir.path().join("file2.txt");
        std::fs::write(&file1_path, "modified1\n").unwrap();
        std::fs::write(&file2_path, "modified2\n").unwrap();

        // Vérifier que les fichiers sont modifiés
        assert_eq!(std::fs::read_to_string(&file1_path).unwrap(), "modified1\n");
        assert_eq!(std::fs::read_to_string(&file2_path).unwrap(), "modified2\n");

        // Discard toutes les modifications
        discard_all(&repo).unwrap();

        // Vérifier que les fichiers sont restaurés
        assert_eq!(std::fs::read_to_string(&file1_path).unwrap(), "content1\n");
        assert_eq!(std::fs::read_to_string(&file2_path).unwrap(), "content2\n");
    }

    #[test]
    fn test_discard_all_removes_untracked_file() {
        let (temp_dir, repo) = create_test_repo();
        commit_file(&repo, "tracked.txt", "tracked\n", "Initial commit");

        let file_path = temp_dir.path().join("untracked.txt");
        std::fs::write(&file_path, "temp\n").unwrap();
        assert!(file_path.exists());

        discard_all(&repo).unwrap();

        assert!(!file_path.exists());
    }

    #[test]
    fn test_discard_file_removes_untracked_file() {
        let (temp_dir, repo) = create_test_repo();
        commit_file(&repo, "tracked.txt", "tracked\n", "Initial commit");

        let file_path = temp_dir.path().join("untracked.txt");
        std::fs::write(&file_path, "temp\n").unwrap();
        assert!(file_path.exists());

        discard_file(&repo, "untracked.txt").unwrap();

        assert!(!file_path.exists());
    }
}
