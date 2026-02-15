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
    use std::fs;
    use std::path::Path;

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Configuration minimale
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        (temp_dir, repo)
    }

    fn create_initial_commit(repo: &Repository, temp_dir: &Path) {
        // Créer un fichier initial
        let file_path = temp_dir.join("test.txt");
        fs::write(&file_path, "initial content\n").unwrap();

        // Stager et committer
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = repo.signature().unwrap();

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn test_discard_file() {
        let (temp_dir, repo) = create_test_repo();
        create_initial_commit(&repo, temp_dir.path());

        // Modifier le fichier
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "modified content\n").unwrap();

        // Vérifier que le fichier est modifié
        let content_before = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content_before, "modified content\n");

        // Discard les modifications
        discard_file(&repo, "test.txt").unwrap();

        // Vérifier que le fichier est restauré
        let content_after = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content_after, "initial content\n");
    }

    #[test]
    fn test_discard_all() {
        let (temp_dir, repo) = create_test_repo();
        create_initial_commit(&repo, temp_dir.path());

        // Créer un deuxième fichier initial
        let file2_path = temp_dir.path().join("test2.txt");
        fs::write(&file2_path, "file2 content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test2.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = repo.signature().unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Add test2.txt",
            &tree,
            &[&parent],
        )
        .unwrap();

        // Modifier les deux fichiers
        let file1_path = temp_dir.path().join("test.txt");
        fs::write(&file1_path, "modified content 1\n").unwrap();
        fs::write(&file2_path, "modified content 2\n").unwrap();

        // Vérifier que les fichiers sont modifiés
        assert_eq!(
            fs::read_to_string(&file1_path).unwrap(),
            "modified content 1\n"
        );
        assert_eq!(
            fs::read_to_string(&file2_path).unwrap(),
            "modified content 2\n"
        );

        // Discard toutes les modifications
        discard_all(&repo).unwrap();

        // Vérifier que les fichiers sont restaurés
        assert_eq!(
            fs::read_to_string(&file1_path).unwrap(),
            "initial content\n"
        );
        assert_eq!(fs::read_to_string(&file2_path).unwrap(), "file2 content\n");
    }
}
