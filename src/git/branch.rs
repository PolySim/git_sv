use git2::{Branch, BranchType, Repository};

use crate::error::Result;

/// Informations sur une branche.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    /// Dernier message de commit sur cette branche.
    pub last_commit_message: Option<String>,
    /// Date du dernier commit.
    pub last_commit_date: Option<i64>,
    /// Nombre de commits d'avance/retard par rapport à la branche tracking.
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
}

impl BranchInfo {
    /// Crée un BranchInfo simple (pour compatibilité avec le code existant).
    pub fn simple(name: String, is_head: bool, is_remote: bool) -> Self {
        Self {
            name,
            is_head,
            is_remote,
            last_commit_message: None,
            last_commit_date: None,
            ahead: None,
            behind: None,
        }
    }
}

/// Construit les informations d'une branche locale.
///
/// Cette fonction factorise la logique commune entre list_branches() et list_all_branches().
fn build_local_branch_info(
    repo: &Repository,
    branch: &Branch,
    is_head: bool,
) -> Result<BranchInfo> {
    let name = branch.name()?.unwrap_or("???").to_string();

    // Récupérer les métadonnées du dernier commit.
    let (last_msg, last_date, ahead, behind) = if let Ok(reference) = branch.get().peel_to_commit()
    {
        let msg = reference.summary().map(|s| s.to_string());
        let date = Some(reference.time().seconds());

        // Calculer ahead/behind si tracking.
        // CORRECTION: Un seul appel à graph_ahead_behind au lieu de deux
        let (ahead_count, behind_count) = if let Ok(upstream) = branch.upstream() {
            if let Ok(upstream_ref) = upstream.get().peel_to_commit() {
                repo.graph_ahead_behind(reference.id(), upstream_ref.id())
                    .map(|(a, b)| (Some(a), Some(b)))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        (msg, date, ahead_count, behind_count)
    } else {
        (None, None, None, None)
    };

    Ok(BranchInfo {
        name,
        is_head,
        is_remote: false,
        last_commit_message: last_msg,
        last_commit_date: last_date,
        ahead,
        behind,
    })
}

/// Construit les informations d'une branche remote.
fn build_remote_branch_info(branch: &Branch) -> Result<BranchInfo> {
    let name = branch.name()?.unwrap_or("???").to_string();

    // Récupérer les métadonnées du dernier commit.
    let (last_msg, last_date) = if let Ok(reference) = branch.get().peel_to_commit() {
        let msg = reference.summary().map(|s| s.to_string());
        let date = Some(reference.time().seconds());
        (msg, date)
    } else {
        (None, None)
    };

    Ok(BranchInfo {
        name,
        is_head: false,
        is_remote: true,
        last_commit_message: last_msg,
        last_commit_date: last_date,
        ahead: None,
        behind: None,
    })
}

/// Liste toutes les branches locales du repository.
pub fn list_branches(repo: &Repository) -> Result<Vec<BranchInfo>> {
    // Utilise list_all_branches et filtre pour ne garder que les locales
    let (local_branches, _) = list_all_branches(repo)?;
    Ok(local_branches)
}

/// Liste toutes les branches (locales et remote).
pub fn list_all_branches(repo: &Repository) -> Result<(Vec<BranchInfo>, Vec<BranchInfo>)> {
    let mut local_branches = Vec::new();
    let mut remote_branches = Vec::new();

    let head_ref = repo.head().ok();
    let head_name = head_ref
        .as_ref()
        .and_then(|h| h.shorthand().map(String::from));

    // Branches locales.
    for branch_result in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch_result?;
        let name = branch.name()?.unwrap_or("???").to_string();
        let is_head = head_name.as_deref() == Some(&name);

        let branch_info = build_local_branch_info(repo, &branch, is_head)?;
        local_branches.push(branch_info);
    }

    // Branches remote.
    for branch_result in repo.branches(Some(BranchType::Remote))? {
        let (branch, _) = branch_result?;
        let branch_info = build_remote_branch_info(&branch)?;
        remote_branches.push(branch_info);
    }

    Ok((local_branches, remote_branches))
}

/// Crée une nouvelle branche à partir de HEAD.
pub fn create_branch(repo: &Repository, name: &str) -> Result<()> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    repo.branch(name, &commit, false)?;
    Ok(())
}

/// Checkout une branche existante.
pub fn checkout_branch(repo: &Repository, name: &str) -> Result<()> {
    let refname = format!("refs/heads/{}", name);
    let obj = repo.revparse_single(&refname)?;
    repo.checkout_tree(&obj, None)?;
    repo.set_head(&refname)?;
    Ok(())
}

/// Supprime une branche locale.
pub fn delete_branch(repo: &Repository, name: &str) -> Result<()> {
    let mut branch = repo.find_branch(name, BranchType::Local)?;
    branch.delete()?;
    Ok(())
}

/// Renomme une branche locale.
pub fn rename_branch(repo: &Repository, old_name: &str, new_name: &str) -> Result<()> {
    let mut branch = repo.find_branch(old_name, BranchType::Local)?;
    branch.rename(new_name, false)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;

    #[test]
    fn test_list_branches() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "content", "Initial commit");

        // Créer une branche supplémentaire
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &head, false).unwrap();

        let branches = list_branches(&repo).unwrap();

        // Devrait avoir 2 branches
        assert_eq!(branches.len(), 2);

        // Vérifier que main est la branche HEAD
        let main_branch = branches.iter().find(|b| b.name == "main").unwrap();
        assert!(main_branch.is_head);

        // Vérifier que feature n'est pas HEAD
        let feature_branch = branches.iter().find(|b| b.name == "feature").unwrap();
        assert!(!feature_branch.is_head);
    }

    #[test]
    fn test_list_all_branches() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "content", "Initial commit");

        let (local, remote) = list_all_branches(&repo).unwrap();

        // Devrait avoir 1 branche locale (main)
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].name, "main");

        // Pas de remote configuré
        assert!(remote.is_empty());
    }

    #[test]
    fn test_create_branch() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "content", "Initial commit");

        // Créer une nouvelle branche
        create_branch(&repo, "new-feature").unwrap();

        // Vérifier que la branche existe
        let branches = list_branches(&repo).unwrap();
        assert_eq!(branches.len(), 2);

        let new_branch = branches.iter().find(|b| b.name == "new-feature").unwrap();
        assert!(!new_branch.is_head); // N'est pas HEAD
        assert!(!new_branch.is_remote);
    }

    #[test]
    fn test_checkout_branch() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "content", "Initial commit");

        // Créer une nouvelle branche
        create_branch(&repo, "feature").unwrap();

        // Checkout sur la nouvelle branche
        checkout_branch(&repo, "feature").unwrap();

        // Vérifier que HEAD pointe sur feature
        let branches = list_branches(&repo).unwrap();
        let feature_branch = branches.iter().find(|b| b.name == "feature").unwrap();
        assert!(feature_branch.is_head);
    }

    #[test]
    fn test_delete_branch() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "content", "Initial commit");

        // Créer une branche
        create_branch(&repo, "to-delete").unwrap();

        // Vérifier qu'elle existe
        let branches = list_branches(&repo).unwrap();
        assert_eq!(branches.len(), 2);

        // Supprimer la branche
        delete_branch(&repo, "to-delete").unwrap();

        // Vérifier qu'elle n'existe plus
        let branches = list_branches(&repo).unwrap();
        assert_eq!(branches.len(), 1);
        assert!(branches.iter().all(|b| b.name != "to-delete"));
    }

    #[test]
    fn test_rename_branch() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "content", "Initial commit");

        // Créer une branche
        create_branch(&repo, "old-name").unwrap();

        // Renommer la branche
        rename_branch(&repo, "old-name", "new-name").unwrap();

        // Vérifier le renommage
        let branches = list_branches(&repo).unwrap();
        assert!(branches.iter().any(|b| b.name == "new-name"));
        assert!(!branches.iter().any(|b| b.name == "old-name"));
    }
}
