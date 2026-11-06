use git2::Repository;

use crate::error::{GitSvError, Result};

/// Recupere le nom court de la branche courante (HEAD).
pub fn get_current_branch_name(repo: &Repository) -> String {
    match repo.head() {
        Ok(head) => {
            if let Ok(name) = head.shorthand() {
                name.to_string()
            } else {
                head.target()
                    .map(|oid| format!("{:.7}", oid))
                    .unwrap_or_else(|| "HEAD".to_string())
            }
        }
        Err(_) => "HEAD".to_string(),
    }
}

/// Verifie si le repository est en etat de merge.
pub fn is_merging(repo: &Repository) -> bool {
    repo.path().join("MERGE_HEAD").exists()
}

/// Annule le merge en cours.
pub fn abort_merge(repo: &Repository) -> Result<()> {
    run_operation(repo, "--abort")
}

fn run_operation(repo: &Repository, argument: &str) -> Result<()> {
    use git2::RepositoryState;
    let operation = match repo.state() {
        RepositoryState::Merge => "merge",
        RepositoryState::Rebase
        | RepositoryState::RebaseInteractive
        | RepositoryState::RebaseMerge
        | RepositoryState::ApplyMailboxOrRebase => "rebase",
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence => "cherry-pick",
        RepositoryState::Revert | RepositoryState::RevertSequence => "revert",
        RepositoryState::ApplyMailbox => "am",
        _ => {
            return Err(GitSvError::Other(
                "Aucune opération Git à continuer ou annuler".into(),
            ))
        }
    };
    let mut command = std::process::Command::new("git");
    command
        .args([operation, argument])
        .env("GIT_EDITOR", "true")
        .current_dir(repo.workdir().unwrap_or(repo.path()));
    let output = crate::git::helpers::run_command_with_timeout(
        command,
        "résolution Git",
        std::time::Duration::from_secs(300),
    )?;
    repo.index()?.read(true)?;
    if !output.status.success() {
        return Err(GitSvError::OperationFailed {
            operation: "résolution Git",
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

/// Finalise le merge en creant le commit de merge.
pub fn finalize_merge(repo: &Repository, message: &str) -> Result<()> {
    if repo.state() != git2::RepositoryState::Merge {
        return run_operation(repo, "--continue");
    }
    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

    index.read(true)?;
    if index.has_conflicts() {
        let remaining: Vec<String> = index
            .conflicts()
            .map_err(|e| GitSvError::Other(format!("Impossible de lister les conflits: {}", e)))?
            .filter_map(|c| c.ok())
            .filter_map(|c| {
                c.our
                    .or(c.their)
                    .or(c.ancestor)
                    .and_then(|e| String::from_utf8(e.path).ok())
            })
            .collect();

        return Err(GitSvError::Other(format!(
            "Des conflits non resolus subsistent dans l'index : {:?}. Resolvez tous les fichiers avant de finaliser.",
            remaining
        )));
    }

    let signature = repo
        .signature()
        .map_err(|e| GitSvError::Other(format!("Impossible d'obtenir la signature: {}", e)))?;

    let head = repo
        .head()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a HEAD: {}", e)))?;
    let head_commit = head
        .peel_to_commit()
        .map_err(|e| GitSvError::Other(format!("Impossible de resoudre HEAD: {}", e)))?;

    let tree_id = index
        .write_tree()
        .map_err(|e| GitSvError::Other(format!("Impossible d'ecrire l'arbre: {}", e)))?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|e| GitSvError::Other(format!("Impossible de trouver l'arbre: {}", e)))?;

    let merge_heads = std::fs::read_to_string(repo.path().join("MERGE_HEAD"))?;
    let mut parents = vec![head_commit];
    for line in merge_heads.lines().filter(|line| !line.is_empty()) {
        parents.push(repo.find_commit(git2::Oid::from_str(line)?)?);
    }
    if parents.len() < 2 {
        return Err(GitSvError::Other(
            "MERGE_HEAD ne contient aucun parent".into(),
        ));
    }
    let parent_refs = parents.iter().collect::<Vec<_>>();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parent_refs,
    )
    .map_err(|e| GitSvError::Other(format!("Impossible de creer le commit: {}", e)))?;

    repo.cleanup_state()
        .map_err(|e| GitSvError::Other(format!("Impossible de nettoyer l'etat de merge: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::conflict::{
        list_conflict_files, resolve_file_with_strategy, ConflictResolution,
    };
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    fn conflicting_branches() -> (tempfile::TempDir, Repository) {
        let (directory, repo) = create_test_repo();
        commit_file(&repo, "local.txt", "base\n", "base");
        let base = commit_file(&repo, "file.txt", "base\n", "base");
        repo.branch("feature", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        commit_file(&repo, "file.txt", "feature\n", "feature");
        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        commit_file(&repo, "file.txt", "main\n", "main");
        (directory, repo)
    }

    #[test]
    fn abort_merge_preserves_unrelated_uncommitted_content() {
        let (directory, repo) = conflicting_branches();
        std::fs::write(directory.path().join("local.txt"), "local changes\n").unwrap();
        crate::git::merge::merge_branch_with_result(&repo, "feature").unwrap();
        assert!(repo.index().unwrap().has_conflicts());
        abort_merge(&repo).unwrap();
        assert_eq!(
            std::fs::read_to_string(directory.path().join("local.txt")).unwrap(),
            "local changes\n"
        );
        assert_eq!(repo.state(), git2::RepositoryState::Clean);
    }

    #[test]
    fn merge_resolution_uses_repository_path_and_clears_index_conflicts() {
        let (_directory, repo) = conflicting_branches();
        crate::git::merge::merge_branch_with_result(&repo, "feature").unwrap();
        let files = list_conflict_files(&repo).unwrap();
        assert_eq!(files.len(), 1);
        assert!(!files[0].is_resolved);
        resolve_file_with_strategy(&repo, &files[0].path, ConflictResolution::Theirs).unwrap();
        assert!(!repo.index().unwrap().has_conflicts());
        finalize_merge(&repo, "resolved").unwrap();
        assert_eq!(
            repo.head()
                .unwrap()
                .peel_to_commit()
                .unwrap()
                .parent_count(),
            2
        );
        assert_eq!(repo.state(), git2::RepositoryState::Clean);
    }

    #[test]
    fn resolving_rebase_continues_the_native_operation() {
        let (_directory, repo) = conflicting_branches();
        let outcome = crate::git::rebase::rebase_branch_with_result(&repo, "feature").unwrap();
        assert!(
            matches!(outcome, crate::git::conflict::MergeResult::Conflicts(ref files) if files.len() == 1)
        );
        assert!(repo.index().unwrap().has_conflicts());
        resolve_file_with_strategy(&repo, "file.txt", ConflictResolution::Theirs).unwrap();
        finalize_merge(&repo, "ignored during rebase").unwrap();
        assert_eq!(repo.state(), git2::RepositoryState::Clean);
        let head = repo.head().unwrap();
        assert_eq!(head.shorthand().unwrap(), "main");
        assert_eq!(head.peel_to_commit().unwrap().parent_count(), 1);
    }
}
