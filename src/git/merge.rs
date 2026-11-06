//! Opérations de merge entre branches.

use git2::{Repository, Signature};

use crate::error::{GitSvError, Result};
use crate::git::conflict::{list_conflict_files, MergeResult};

/// Effectue un merge et retourne un résultat typé pour gérer les conflits.
pub fn merge_branch_with_result(repo: &Repository, branch_name: &str) -> Result<MergeResult> {
    let refname = format!("refs/heads/{}", branch_name);
    let reference = repo.find_reference(&refname)?;
    let annotated = repo.reference_to_annotated_commit(&reference)?;
    let source_commit = reference.peel_to_commit()?;
    let head_commit = repo.head()?.peel_to_commit()?;

    let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    if analysis.is_up_to_date() {
        return Ok(MergeResult::UpToDate);
    }

    if analysis.is_fast_forward() || analysis.is_normal() {
        // Le checkout sûr de libgit2 protège les changements locaux avant toute mutation de HEAD.
        repo.merge(&[&annotated], None, None)?;

        // Vérifier s'il y a des conflits.
        let mut index = repo.index()?;
        if index.has_conflicts() {
            // Lister les fichiers en conflit
            let conflict_files = list_conflict_files(repo)?;
            return Ok(MergeResult::Conflicts(conflict_files));
        }

        let signature = repo
            .signature()
            .or_else(|_| Signature::now("git_sv", "git_sv@local"))?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Merge branch '{}'", branch_name),
            &tree,
            &[&head_commit, &source_commit],
        )?;

        repo.cleanup_state()?;

        return Ok(MergeResult::Success);
    }

    Err(GitSvError::Other("Merge impossible".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::commit_file;
    use git2::build::CheckoutBuilder;

    #[test]
    fn fast_forward_merge_refuses_to_overwrite_local_changes() {
        let (directory, repo) = crate::git::tests::test_utils::create_test_repo();
        let base = commit_file(&repo, "file.txt", "base\n", "base");
        repo.branch("feature", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        commit_file(&repo, "file.txt", "feature\n", "feature");
        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();
        std::fs::write(directory.path().join("file.txt"), "local work\n").unwrap();

        assert!(merge_branch_with_result(&repo, "feature").is_err());
        assert_eq!(repo.head().unwrap().target(), Some(base));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("file.txt")).unwrap(),
            "local work\n"
        );
    }

    #[test]
    fn merge_preserves_unrelated_unstaged_changes() {
        let (directory, repo) = crate::git::tests::test_utils::create_test_repo();
        commit_file(&repo, "local.txt", "base\n", "base");
        let base = commit_file(&repo, "file.txt", "base\n", "base");
        repo.branch("feature", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        commit_file(&repo, "file.txt", "feature\n", "feature");
        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();
        std::fs::write(directory.path().join("local.txt"), "local work\n").unwrap();

        merge_branch_with_result(&repo, "feature").unwrap();
        assert_eq!(
            std::fs::read_to_string(directory.path().join("local.txt")).unwrap(),
            "local work\n"
        );
        assert!(repo
            .status_file(std::path::Path::new("local.txt"))
            .unwrap()
            .is_wt_modified());
    }

    #[test]
    fn merge_creates_commit_even_when_fast_forward_is_possible() {
        let (_tmp, repo) = crate::git::tests::test_utils::create_test_repo();

        let base_oid = commit_file(&repo, "file.txt", "base\n", "base");

        let base_commit = repo.find_commit(base_oid).expect("base commit introuvable");
        repo.branch("feature", &base_commit, false)
            .expect("impossible de créer la branche feature");

        repo.set_head("refs/heads/feature")
            .expect("impossible de checkout feature");
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .expect("impossible de checkout feature");

        let feature_oid = commit_file(&repo, "file.txt", "feature\n", "feature");

        repo.set_head("refs/heads/main")
            .expect("impossible de checkout main");
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .expect("impossible de checkout main");

        let result = merge_branch_with_result(&repo, "feature").expect("merge échoué");
        assert!(matches!(result, MergeResult::Success));

        let head = repo
            .head()
            .expect("HEAD introuvable")
            .peel_to_commit()
            .expect("commit HEAD introuvable");

        assert_eq!(head.parent_count(), 2);
        assert_eq!(head.parent_id(0).expect("parent 0"), base_oid);
        assert_eq!(head.parent_id(1).expect("parent 1"), feature_oid);
        assert_eq!(
            head.summary().unwrap().unwrap_or(""),
            "Merge branch 'feature'"
        );
    }
}
