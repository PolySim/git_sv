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
    let head = repo.head()?.peel_to_commit()?;
    if head.id() != first_commit_oid && !repo.graph_descendant_of(head.id(), first_commit_oid)? {
        return Err(git2::Error::from_str(
            "Le commit sélectionné ne fait pas partie de la branche courante",
        )
        .into());
    }
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

    let status = super::helpers::clear_repository_environment(&mut command)
        .current_dir(repo_path)
        .status()?;
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

    let previous_head = repo.head()?.target();
    let mut command = Command::new("git");
    command
        .args(["rebase", "--", branch_name])
        .current_dir(repo_path);
    let output = super::helpers::run_command_with_timeout(
        command,
        "git rebase",
        std::time::Duration::from_secs(300),
    )?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    repo.index()?.read(true)?;

    if output.status.success() {
        if repo.head()?.target() == previous_head {
            Ok(MergeResult::UpToDate)
        } else {
            Ok(MergeResult::Success)
        }
    } else {
        let conflicts = list_conflict_files(repo)?;
        if !conflicts.is_empty() {
            return Ok(MergeResult::Conflicts(conflicts));
        }
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
        assert_eq!(parent.summary().unwrap().unwrap_or(""), "main");
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
            repo.head()
                .unwrap()
                .peel_to_commit()
                .unwrap()
                .summary()
                .unwrap(),
            Some("changed")
        );
    }
}
