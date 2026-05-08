//! Opérations de rebase entre branches.

use std::process::Command;

use git2::Repository;

use crate::error::Result;
use crate::git::conflict::{list_conflict_files, MergeResult};

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
}
