//! Opérations de rebase entre branches.

use std::path::Path;
use std::process::Command;

use git2::{Oid, Repository, RepositoryState};

use crate::error::Result;
use crate::git::conflict::{list_conflict_files, MergeResult};

/// Résultat du retour d'un rebase interactif lancé dans l'éditeur utilisateur.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractiveRebaseResult {
    Completed,
    Interrupted(Vec<crate::git::conflict::ConflictFile>),
    Cancelled,
}

/// Lance un rebase interactif incluant le commit sélectionné.
///
/// La commande hérite du terminal courant afin que Git ouvre l'éditeur
/// configuré par l'utilisateur.
pub fn interactive_rebase_from(
    repo_path: &Path,
    first_commit_oid: Oid,
) -> Result<InteractiveRebaseResult> {
    interactive_rebase_from_with_editor(repo_path, first_commit_oid, None)
}

fn interactive_rebase_from_with_editor(
    repo_path: &Path,
    first_commit_oid: Oid,
    sequence_editor: Option<&str>,
) -> Result<InteractiveRebaseResult> {
    let repo = Repository::discover(repo_path)?;
    let first_commit = repo.find_commit(first_commit_oid)?;
    let mut command = Command::new("git");
    command.arg("rebase").arg("--interactive");
    if let Some(editor) = sequence_editor {
        command.env("GIT_SEQUENCE_EDITOR", editor);
    }
    if first_commit.parent_count() == 0 {
        command.arg("--root");
    } else {
        command.arg(first_commit.parent_id(0)?.to_string());
    }

    let status = command.current_dir(repo_path).status()?;
    if status.success() {
        return Ok(InteractiveRebaseResult::Completed);
    }

    let reopened = Repository::discover(repo_path)?;
    if matches!(
        reopened.state(),
        RepositoryState::Rebase
            | RepositoryState::RebaseInteractive
            | RepositoryState::RebaseMerge
            | RepositoryState::ApplyMailbox
            | RepositoryState::ApplyMailboxOrRebase
    ) {
        return Ok(InteractiveRebaseResult::Interrupted(
            list_conflict_files(&reopened).unwrap_or_default(),
        ));
    }

    Ok(InteractiveRebaseResult::Cancelled)
}

/// Effectue un rebase de la branche courante sur une autre branche.
pub fn rebase_branch_with_result(repo: &Repository, branch_name: &str) -> Result<MergeResult> {
    let repo_path = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Impossible de trouver le chemin du repository"))?;

    let output = Command::new("git")
        .args(["rebase", branch_name])
        .current_dir(repo_path)
        .output()
        .map_err(|e| git2::Error::from_str(&format!("Erreur exécuter git rebase: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr).to_lowercase();

    if output.status.success() {
        if combined.contains("up to date") || combined.contains("à jour") {
            Ok(MergeResult::UpToDate)
        } else {
            Ok(MergeResult::Success)
        }
    } else if combined.contains("conflict") || combined.contains("conflit") {
        Ok(MergeResult::Conflicts(list_conflict_files(repo)?))
    } else {
        Err(git2::Error::from_str(&format!("Erreur git rebase: {}", stderr)).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit, commit_file, create_file, create_test_repo};
    use git2::build::CheckoutBuilder;
    use std::path::Path;

    #[test]
    fn test_rebase_branch_with_result_success() {
        let (_tmp, repo) = create_test_repo();

        let base_oid = commit_file(&repo, "file.txt", "base\n", "base");
        let base_commit = repo.find_commit(base_oid).unwrap();
        repo.branch("feature", &base_commit, false).unwrap();

        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();
        commit_file(&repo, "feature.txt", "feature\n", "feature");

        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();
        commit_file(&repo, "main.txt", "main\n", "main");

        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();

        let result = rebase_branch_with_result(&repo, "main").unwrap();
        assert!(matches!(result, MergeResult::Success));

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let parent = head.parent(0).unwrap();
        assert_eq!(parent.summary().unwrap_or(""), "main");
    }

    #[test]
    fn test_rebase_branch_with_result_conflicts() {
        let (_tmp, repo) = create_test_repo();

        let base_oid = commit_file(&repo, "file.txt", "base\n", "base");
        let base_commit = repo.find_commit(base_oid).unwrap();
        repo.branch("feature", &base_commit, false).unwrap();

        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();
        create_file(&repo, "file.txt", "feature\n");
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        commit(&repo, "feature");

        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();
        create_file(&repo, "file.txt", "main\n");
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        commit(&repo, "main");

        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();

        let result = rebase_branch_with_result(&repo, "main").unwrap();
        assert!(matches!(result, MergeResult::Conflicts(_)));

        repo.cleanup_state().unwrap();
    }

    #[test]
    fn test_interactive_rebase_can_use_configured_sequence_editor() {
        let (directory, repo) = create_test_repo();
        commit_file(&repo, "file.txt", "base\n", "base");
        let selected = commit_file(&repo, "file.txt", "changed\n", "changed");

        let result =
            interactive_rebase_from_with_editor(directory.path(), selected, Some("true")).unwrap();

        assert_eq!(result, InteractiveRebaseResult::Completed);
        assert_eq!(
            repo.head().unwrap().peel_to_commit().unwrap().summary(),
            Some("changed")
        );
    }
}
