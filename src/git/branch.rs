use git2::{BranchType, Repository};

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

/// Liste toutes les branches locales du repository.
pub fn list_branches(repo: &Repository) -> Result<Vec<BranchInfo>> {
    let mut branches = Vec::new();

    let head_ref = repo.head().ok();
    let head_name = head_ref
        .as_ref()
        .and_then(|h| h.shorthand().map(String::from));

    for branch_result in repo.branches(Some(BranchType::Local))? {
        let (branch, _branch_type) = branch_result?;
        let name = branch.name()?.unwrap_or("???").to_string();
        let is_head = head_name.as_deref() == Some(&name);

        // Récupérer les métadonnées du dernier commit.
        let (last_msg, last_date, ahead, behind) =
            if let Ok(reference) = branch.get().peel_to_commit() {
                let msg = reference.summary().map(|s| s.to_string());
                let date = Some(reference.time().seconds());

                // Calculer ahead/behind si tracking.
                let (ahead_count, behind_count) = if let Ok(upstream) = branch.upstream() {
                    if let Ok(upstream_ref) = upstream.get().peel_to_commit() {
                        let ahead = repo
                            .graph_ahead_behind(reference.id(), upstream_ref.id())
                            .map(|(a, _)| a)
                            .unwrap_or(0);
                        let behind = repo
                            .graph_ahead_behind(reference.id(), upstream_ref.id())
                            .map(|(_, b)| b)
                            .unwrap_or(0);
                        (Some(ahead), Some(behind))
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

        branches.push(BranchInfo {
            name,
            is_head,
            is_remote: false,
            last_commit_message: last_msg,
            last_commit_date: last_date,
            ahead,
            behind,
        });
    }

    Ok(branches)
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

        // Récupérer les métadonnées du dernier commit.
        let (last_msg, last_date, ahead, behind) =
            if let Ok(reference) = branch.get().peel_to_commit() {
                let msg = reference.summary().map(|s| s.to_string());
                let date = Some(reference.time().seconds());

                // Calculer ahead/behind si tracking.
                let (ahead_count, behind_count) = if let Ok(upstream) = branch.upstream() {
                    if let Ok(upstream_ref) = upstream.get().peel_to_commit() {
                        let ahead = repo
                            .graph_ahead_behind(reference.id(), upstream_ref.id())
                            .map(|(a, _)| a)
                            .unwrap_or(0);
                        let behind = repo
                            .graph_ahead_behind(reference.id(), upstream_ref.id())
                            .map(|(_, b)| b)
                            .unwrap_or(0);
                        (Some(ahead), Some(behind))
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

        local_branches.push(BranchInfo {
            name,
            is_head,
            is_remote: false,
            last_commit_message: last_msg,
            last_commit_date: last_date,
            ahead,
            behind,
        });
    }

    // Branches remote.
    for branch_result in repo.branches(Some(BranchType::Remote))? {
        let (branch, _) = branch_result?;
        let name = branch.name()?.unwrap_or("???").to_string();

        // Récupérer les métadonnées du dernier commit.
        let (last_msg, last_date) = if let Ok(reference) = branch.get().peel_to_commit() {
            let msg = reference.summary().map(|s| s.to_string());
            let date = Some(reference.time().seconds());
            (msg, date)
        } else {
            (None, None)
        };

        remote_branches.push(BranchInfo {
            name,
            is_head: false,
            is_remote: true,
            last_commit_message: last_msg,
            last_commit_date: last_date,
            ahead: None,
            behind: None,
        });
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
