use git2::{BranchType, Repository};

use crate::error::Result;

/// Informations sur une branche.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
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
        let name = branch
            .name()?
            .unwrap_or("???")
            .to_string();
        let is_head = head_name.as_deref() == Some(&name);

        branches.push(BranchInfo {
            name,
            is_head,
            is_remote: false,
        });
    }

    Ok(branches)
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
