use git2::Repository;

use crate::error::{GitSvError, Result};
use crate::git::conflict::{list_conflict_files, MergeResult};

/// Effectue un merge de la branche source dans la branche courante.
pub fn merge_branch(repo: &Repository, branch_name: &str) -> Result<()> {
    let refname = format!("refs/heads/{}", branch_name);
    let reference = repo.find_reference(&refname)?;
    let annotated = repo.reference_to_annotated_commit(&reference)?;

    let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    if analysis.is_up_to_date() {
        return Ok(()); // Rien à faire.
    }

    if analysis.is_fast_forward() {
        // Fast-forward : on avance simplement le pointeur.
        let target_oid = reference
            .target()
            .ok_or_else(|| GitSvError::Other("Référence invalide".into()))?;
        let mut head_ref = repo.head()?;
        head_ref.set_target(target_oid, "fast-forward merge")?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        return Ok(());
    }

    if analysis.is_normal() {
        // Merge normal : on laisse git2 faire le merge dans l'index.
        repo.merge(&[&annotated], None, None)?;

        // Vérifier s'il y a des conflits.
        let index = repo.index()?;
        if index.has_conflicts() {
            return Err(GitSvError::Other(
                "Conflits détectés. Résolvez-les avant de committer.".into(),
            ));
        }

        // Le merge commit sera créé par l'utilisateur via la commande commit.
        return Ok(());
    }

    Err(GitSvError::Other("Merge impossible".into()))
}

/// Effectue un merge et retourne un résultat typé pour gérer les conflits.
pub fn merge_branch_with_result(repo: &Repository, branch_name: &str) -> Result<MergeResult> {
    let refname = format!("refs/heads/{}", branch_name);
    let reference = repo.find_reference(&refname)?;
    let annotated = repo.reference_to_annotated_commit(&reference)?;

    let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    if analysis.is_up_to_date() {
        return Ok(MergeResult::UpToDate);
    }

    if analysis.is_fast_forward() {
        // Fast-forward : on avance simplement le pointeur.
        let target_oid = reference
            .target()
            .ok_or_else(|| GitSvError::Other("Référence invalide".into()))?;
        let mut head_ref = repo.head()?;
        head_ref.set_target(target_oid, "fast-forward merge")?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        return Ok(MergeResult::FastForward);
    }

    if analysis.is_normal() {
        // Merge normal : on laisse git2 faire le merge dans l'index.
        repo.merge(&[&annotated], None, None)?;

        // Vérifier s'il y a des conflits.
        let index = repo.index()?;
        if index.has_conflicts() {
            // Lister les fichiers en conflit
            let conflict_files = list_conflict_files(repo)?;
            return Ok(MergeResult::Conflicts(conflict_files));
        }

        return Ok(MergeResult::Success);
    }

    Err(GitSvError::Other("Merge impossible".into()))
}
