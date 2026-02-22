use git2::Repository;

use crate::error::Result;

/// Informations sur un worktree.
#[derive(Debug, Clone, Default)]
pub struct WorktreeInfo {
    /// Nom du worktree.
    pub name: String,
    /// Chemin absolu du worktree.
    pub path: String,
    /// Branche associée (si applicable).
    pub branch: Option<String>,
    /// Est-ce le worktree principal ?
    pub is_main: bool,
}

/// Liste tous les worktrees du repository.
pub fn list_worktrees(repo: &Repository) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();

    // Worktree principal.
    if let Some(path) = repo.workdir() {
        let branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from));
        worktrees.push(WorktreeInfo {
            name: "main".to_string(),
            path: path.display().to_string(),
            branch,
            is_main: true,
        });
    }

    // Worktrees additionnels.
    let wt_names = repo.worktrees()?;
    for name in wt_names.iter() {
        if let Some(name) = name {
            if let Ok(wt) = repo.find_worktree(name) {
                let path = wt.path().display().to_string();
                // Tenter d'ouvrir le worktree pour lire sa branche.
                let branch = if let Ok(wt_repo) = Repository::open(&path) {
                    wt_repo
                        .head()
                        .ok()
                        .and_then(|h| h.shorthand().map(String::from))
                } else {
                    None
                };

                worktrees.push(WorktreeInfo {
                    name: name.to_string(),
                    path,
                    branch,
                    is_main: false,
                });
            }
        }
    }

    Ok(worktrees)
}

/// Crée un nouveau worktree.
pub fn create_worktree(
    repo: &Repository,
    name: &str,
    path: &str,
    branch: Option<&str>,
) -> Result<()> {
    let _reference = if let Some(branch_name) = branch {
        let refname = format!("refs/heads/{}", branch_name);
        Some(repo.find_reference(&refname)?)
    } else {
        None
    };

    repo.worktree(name, std::path::Path::new(path), None)?;
    Ok(())
}

/// Supprime un worktree (prune).
pub fn remove_worktree(repo: &Repository, name: &str) -> Result<()> {
    let wt = repo.find_worktree(name)?;
    // Vérifier que le worktree est prunable.
    if wt.validate().is_ok() {
        wt.prune(None)?;
    }
    Ok(())
}
