use git2::Repository;

use crate::error::{GitSvError, Result};

/// Verifie si le repository a des conflits non resolus.
pub fn has_conflicts(repo: &Repository) -> Result<bool> {
    let index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

    Ok(index.has_conflicts())
}

/// Recupere le nom court de la branche courante (HEAD).
pub fn get_current_branch_name(repo: &Repository) -> String {
    match repo.head() {
        Ok(head) => {
            if let Some(name) = head.shorthand() {
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

/// Recupere le nom de la branche mergee depuis MERGE_HEAD ou un message d'operation.
pub fn get_merge_branch_name(repo: &Repository, operation_msg: Option<&str>) -> String {
    if let Some(msg) = operation_msg {
        if let Some(start) = msg.find('\'') {
            if let Some(end) = msg[start + 1..].find('\'') {
                return msg[start + 1..start + 1 + end].to_string();
            }
        }
    }

    let merge_head_path = repo.path().join("MERGE_HEAD");
    if let Ok(merge_head_content) = std::fs::read_to_string(&merge_head_path) {
        let merge_head_oid = merge_head_content.trim();
        if let Ok(oid) = git2::Oid::from_str(merge_head_oid) {
            if let Ok(branches) = repo.branches(None) {
                for (branch, _) in branches.flatten() {
                    if let Some(target) = branch.get().target() {
                        if target == oid {
                            if let Some(name) = branch.name().ok().flatten() {
                                return name.to_string();
                            }
                        }
                    }
                }
            }
            return format!("{:.7}", oid);
        }
    }

    operation_msg
        .map(|s| s.to_string())
        .unwrap_or_else(|| "MERGE_HEAD".to_string())
}

/// Verifie si le repository est en etat de merge.
pub fn is_merging(repo: &Repository) -> bool {
    repo.path().join("MERGE_HEAD").exists()
}

/// Annule le merge en cours.
pub fn abort_merge(repo: &Repository) -> Result<()> {
    repo.cleanup_state()
        .map_err(|e| GitSvError::Other(format!("Impossible d'annuler le merge: {}", e)))?;

    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    repo.checkout_head(Some(&mut checkout_builder))
        .map_err(|e| GitSvError::Other(format!("Erreur lors du checkout: {}", e)))?;

    Ok(())
}

/// Finalise le merge en creant le commit de merge.
pub fn finalize_merge(repo: &Repository, message: &str) -> Result<()> {
    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

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

    let merge_head_oid = std::fs::read_to_string(repo.path().join("MERGE_HEAD"))
        .ok()
        .and_then(|content| git2::Oid::from_str(content.trim()).ok());

    let merge_commit = merge_head_oid.and_then(|oid| repo.find_commit(oid).ok());

    let _commit_oid = if let Some(ref merge_commit) = merge_commit {
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit, merge_commit],
        )
    } else {
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit],
        )
    }
    .map_err(|e| GitSvError::Other(format!("Impossible de creer le commit: {}", e)))?;

    repo.cleanup_state()
        .map_err(|e| GitSvError::Other(format!("Impossible de nettoyer l'etat de merge: {}", e)))?;

    Ok(())
}
