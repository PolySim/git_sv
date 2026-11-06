//! Suppression des modifications non committées (discard).

use git2::Repository;
use std::fs;

use crate::error::Result;

/// Discard les modifications d'un fichier spécifique (git checkout -- file).
/// Cette opération restaure le fichier à son état dans l'index, sans modifier le staging.
pub fn discard_file(repo: &Repository, file_path: &str) -> Result<()> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Impossible de trouver le chemin du repository"))?;
    let full_path = crate::git::helpers::worktree_path(repo, std::path::Path::new(file_path))?;
    ensure_not_directory(&full_path)?;
    let status = repo.status_file(std::path::Path::new(file_path))?;

    if status.is_wt_new() && !status.is_index_new() {
        fs::remove_file(&full_path)?;

        return Ok(());
    }

    // Restaurer un chemin littéral, même pour un nom contenant des crochets.
    run_git_discard(
        workdir,
        &[
            "--literal-pathspecs",
            "restore",
            "--worktree",
            "--",
            file_path,
        ],
    )
}

fn ensure_not_directory(path: &std::path::Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Err(crate::error::GitSvError::Other(
            "Discard refusé : un dossier pourrait contenir des fichiers à conserver".into(),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn run_git_discard(workdir: &std::path::Path, args: &[&str]) -> Result<()> {
    let mut command = std::process::Command::new("git");
    command.args(args).current_dir(workdir);
    let output = crate::git::helpers::run_command_with_timeout(
        command,
        "discard",
        std::time::Duration::from_secs(30),
    )?;
    if !output.status.success() {
        return Err(crate::error::GitSvError::OperationFailed {
            operation: "discard",
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(())
}

/// Discard toutes les modifications non stagées (git checkout -- .).
/// Conserve le staging, les fichiers ignorés et les dépôts Git imbriqués.
pub fn discard_all(repo: &Repository) -> Result<()> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Working tree introuvable"))?;
    let index = repo.index()?;
    // Vérifier uniquement les fichiers modifiés avant de toucher au working tree.
    let diff = repo.diff_index_to_workdir(Some(&index), None)?;
    for delta in diff.deltas() {
        if let Some(path) = delta.old_file().path() {
            if index
                .get_path(path, 0)
                .is_some_and(|entry| entry.mode != 0o160000)
            {
                let full_path = crate::git::helpers::worktree_path(repo, path)?;
                ensure_not_directory(&full_path)?;
            }
        }
    }
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    checkout_builder.update_index(false);

    repo.checkout_index(None, Some(&mut checkout_builder))?;
    // Un seul -f protège les dépôts imbriqués ; sans -x, Git conserve les ignorés.
    run_git_discard(workdir, &["clean", "-f", "-d"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[cfg(unix)]
    #[test]
    fn discard_does_not_modify_files_through_a_replaced_parent_symlink() {
        for (tracked, sentinel) in [
            (false, b"external sentinel\n".as_slice()),
            (true, b"external sentinel\n".as_slice()),
            (true, b"base\n".as_slice()),
        ] {
            for all in [false, true] {
                let (temp, repo) = create_test_repo();
                commit_file(&repo, "base.txt", "base\n", "base");
                if tracked {
                    commit_file(&repo, "folder/file.txt", "base\n", "tracked");
                    fs::remove_dir_all(temp.path().join("folder")).unwrap();
                }
                let outside = tempfile::TempDir::new().unwrap();
                fs::write(outside.path().join("file.txt"), sentinel).unwrap();
                std::os::unix::fs::symlink(outside.path(), temp.path().join("folder")).unwrap();
                let _ = if all {
                    discard_all(&repo)
                } else {
                    discard_file(&repo, "folder/file.txt")
                };
                assert_eq!(
                    fs::read(outside.path().join("file.txt")).ok(),
                    Some(sentinel.to_vec()),
                    "tracked={tracked}, all={all}"
                );
            }
        }
    }

    #[test]
    fn discard_recreates_missing_tracked_parent_directories() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "folder/nested/file.txt", "base\n", "base");
        fs::remove_dir_all(temp.path().join("folder")).unwrap();
        discard_file(&repo, "folder/nested/file.txt").unwrap();
        assert_eq!(
            fs::read(temp.path().join("folder/nested/file.txt")).unwrap(),
            b"base\n"
        );
    }

    #[test]
    fn discard_all_preserves_ignored_files_inside_untracked_directories() {
        let (directory, repo) = create_test_repo();
        commit_file(&repo, ".gitignore", "*.ignored\n", "ignore");
        fs::create_dir(directory.path().join("output")).unwrap();
        fs::write(
            directory.path().join("output/secret.ignored"),
            b"private data",
        )
        .unwrap();
        fs::write(directory.path().join("output/temporary.txt"), b"discard").unwrap();
        discard_all(&repo).unwrap();
        assert_eq!(
            fs::read(directory.path().join("output/secret.ignored")).unwrap(),
            b"private data"
        );
        assert!(!directory.path().join("output/temporary.txt").exists());
    }

    #[test]
    fn discard_does_not_remove_an_embedded_git_repository() {
        for all in [false, true] {
            let (directory, repo) = create_test_repo();
            commit_file(&repo, "tracked.txt", "tracked\n", "base");
            let nested = git2::Repository::init(directory.path().join("nested")).unwrap();
            fs::write(nested.workdir().unwrap().join("keep.txt"), b"nested data").unwrap();
            let nested_head = fs::read(nested.path().join("HEAD")).unwrap();
            if all {
                discard_all(&repo).unwrap();
            } else {
                let _ = discard_file(&repo, "nested/");
            }
            assert_eq!(
                fs::read(nested.workdir().unwrap().join("keep.txt")).unwrap(),
                b"nested data"
            );
            assert_eq!(fs::read(nested.path().join("HEAD")).unwrap(), nested_head);
        }
    }

    #[test]
    fn discard_refuses_to_replace_a_directory_containing_ignored_data() {
        for all in [false, true] {
            let (directory, repo) = create_test_repo();
            commit_file(&repo, ".gitignore", "*.ignored\n", "ignore");
            commit_file(&repo, "tracked.txt", "base\n", "base");
            fs::remove_file(directory.path().join("tracked.txt")).unwrap();
            fs::create_dir(directory.path().join("tracked.txt")).unwrap();
            let private_path = directory.path().join("tracked.txt/private.ignored");
            fs::write(&private_path, b"private data").unwrap();
            let result = if all {
                discard_all(&repo)
            } else {
                discard_file(&repo, "tracked.txt")
            };
            assert_eq!(fs::read(private_path).unwrap(), b"private data");
            assert!(result.is_err());
        }
    }

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
