//! Gestion des worktrees git.

use std::path::{Path, PathBuf};

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
    /// Est-ce le worktree actuellement ouvert dans l'application ?
    pub is_current: bool,
}

/// Liste tous les worktrees du repository.
pub fn list_worktrees(repo: &Repository) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();

    let current_path = repo.workdir().map(normalized_path);

    // Le worktree principal est le parent du repertoire git commun, meme si
    // l'application a ete ouverte depuis un worktree lie.
    if let Some(path) = main_worktree_path(repo) {
        let main_repo = Repository::open(path).ok();
        let branch = main_repo
            .as_ref()
            .and_then(|repository| repository.head().ok())
            .and_then(|head| head.shorthand().map(String::from));
        let normalized = normalized_path(path);
        worktrees.push(WorktreeInfo {
            name: "main".to_string(),
            path: normalized.display().to_string(),
            branch,
            is_main: true,
            is_current: current_path.as_ref() == Some(&normalized),
        });
    }

    // Worktrees additionnels.
    let wt_names = repo.worktrees()?;
    for name in wt_names.iter().flatten() {
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
                is_current: current_path
                    .as_ref()
                    .is_some_and(|current| *current == normalized_path(Path::new(&path))),
                path,
                branch,
                is_main: false,
            });
        }
    }

    Ok(worktrees)
}

fn normalized_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn main_worktree_path(repo: &Repository) -> Option<&Path> {
    if repo.is_worktree() {
        repo.path().parent()?.parent()?.parent()
    } else {
        repo.workdir()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::repo::GitRepo;
    use crate::state::{AppState, BranchesSection, ViewMode};
    use tempfile::TempDir;

    fn create_repository_with_worktree() -> (TempDir, PathBuf, PathBuf) {
        let temp = TempDir::new().unwrap();
        let main_path = temp.path().join("main");
        let linked_path = temp.path().join("feature-worktree");
        let repository = Repository::init(&main_path).unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let mut index = repository.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repository.find_tree(tree_oid).unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();
        drop(tree);
        repository
            .worktree("feature-worktree", &linked_path, None)
            .unwrap();

        (temp, main_path, linked_path)
    }

    #[test]
    fn test_list_worktrees_marks_the_open_linked_worktree_as_current() {
        let (_temp, main_path, linked_path) = create_repository_with_worktree();
        let repository = Repository::open(&linked_path).unwrap();
        let worktrees = list_worktrees(&repository).unwrap();

        assert_eq!(worktrees.len(), 2);
        assert!(worktrees.iter().any(|worktree| {
            worktree.is_main
                && !worktree.is_current
                && Path::new(&worktree.path) == normalized_path(&main_path)
        }));
        assert!(
            worktrees.iter().any(|worktree| {
                !worktree.is_main
                    && worktree.is_current
                    && Path::new(&worktree.path) == normalized_path(&linked_path)
            }),
            "worktrees={worktrees:#?}, workdir={:?}",
            repository.workdir()
        );
    }

    #[test]
    fn test_app_state_switches_to_linked_worktree() {
        let (_temp, main_path, linked_path) = create_repository_with_worktree();
        let repo = GitRepo::open(main_path.to_string_lossy().as_ref()).unwrap();
        let mut state = AppState::new(repo, main_path.display().to_string()).unwrap();
        state.view_mode = ViewMode::Branches;
        state.branches_view_state.section = BranchesSection::Worktrees;

        state
            .switch_repository(linked_path.to_string_lossy().as_ref())
            .unwrap();

        assert_eq!(Path::new(&state.repo_path), normalized_path(&linked_path));
        assert!(state.repo.repo.is_worktree());
        assert_eq!(state.view_mode, ViewMode::Branches);
        assert_eq!(
            state.branches_view_state.section,
            BranchesSection::Worktrees
        );
        assert!(state
            .branches_view_state
            .worktrees
            .iter()
            .any(|worktree| worktree.is_current && !worktree.is_main));
    }
}
