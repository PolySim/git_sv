//! Lecture du reflog et annulation sûre de la dernière transition de HEAD.

use git2::{Oid, Repository, ResetType};

use crate::error::{GitSvError, Result};

/// Cible d'annulation déduite de la dernière entrée du reflog HEAD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoTarget {
    pub oid: Oid,
    pub description: String,
}

/// Retourne l'ancien HEAD enregistré par la dernière opération Git.
pub fn last_undo_target(repo: &Repository) -> Result<UndoTarget> {
    let reflog = repo.reflog("HEAD")?;
    let entry = reflog
        .get(0)
        .ok_or_else(|| GitSvError::Other("Reflog HEAD vide".into()))?;
    let oid = entry.id_old();
    if oid.is_zero() || repo.find_commit(oid).is_err() {
        return Err(GitSvError::Other(
            "Aucun état précédent annulable dans le reflog".into(),
        ));
    }

    Ok(UndoTarget {
        oid,
        description: entry
            .message()
            .unwrap_or("dernière opération Git")
            .to_string(),
    })
}

/// Replace HEAD sur la cible en conservant les différences dans le working tree.
pub fn undo_to(repo: &Repository, oid: Oid) -> Result<()> {
    let commit = repo.find_commit(oid)?;
    repo.reset(commit.as_object(), ResetType::Mixed, None)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn test_reflog_undo_restores_head_and_preserves_changes() {
        let (_directory, repo) = create_test_repo();
        let first = commit_file(&repo, "file.txt", "first\n", "first");
        let second = commit_file(&repo, "file.txt", "second\n", "second");
        assert_eq!(repo.head().unwrap().target(), Some(second));

        let target = last_undo_target(&repo).unwrap();
        assert_eq!(target.oid, first);
        undo_to(&repo, target.oid).unwrap();

        assert_eq!(repo.head().unwrap().target(), Some(first));
        let worktree_content =
            std::fs::read_to_string(repo.workdir().unwrap().join("file.txt")).unwrap();
        assert_eq!(worktree_content, "second\n");
        assert!(repo
            .status_file(std::path::Path::new("file.txt"))
            .unwrap()
            .is_wt_modified());
    }
}
