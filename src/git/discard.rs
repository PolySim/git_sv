//! Suppression des modifications non committées (discard).

use git2::Repository;

use crate::error::Result;

/// Discard les modifications d'un fichier spécifique (git checkout -- file).
/// Cette opération restaure le fichier à son état dans HEAD.
pub fn discard_file(repo: &Repository, file_path: &str) -> Result<()> {
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    checkout_builder.path(file_path);

    repo.checkout_head(Some(&mut checkout_builder))?;

    Ok(())
}

/// Discard toutes les modifications non stagées (git checkout -- .).
/// Cette opération restaure tous les fichiers modifiés à leur état dans HEAD.
pub fn discard_all(repo: &Repository) -> Result<()> {
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    checkout_builder.remove_untracked(false);

    repo.checkout_head(Some(&mut checkout_builder))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

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
}
