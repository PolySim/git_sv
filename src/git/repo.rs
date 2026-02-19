use git2::{Repository, StatusOptions};

use super::branch::BranchInfo;
use super::commit::CommitInfo;
use super::graph::GraphRow;
use super::stash::StashEntry;
use super::worktree::WorktreeInfo;
use crate::error::Result;

/// Wrapper haut-niveau autour de git2::Repository.
pub struct GitRepo {
    pub repo: Repository,
}

impl GitRepo {
    /// Ouvre le repository git dans le répertoire donné.
    pub fn open(path: &str) -> Result<Self> {
        let repo = Repository::discover(path)?;
        Ok(Self { repo })
    }

    /// Retourne le nom de la branche courante (HEAD).
    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head()?;
        let name = head.shorthand().unwrap_or("HEAD détachée").to_string();
        Ok(name)
    }

    /// Retourne la liste des commits (log) depuis HEAD.
    pub fn log(&self, max_count: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;

        let mut commits = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if i >= max_count {
                break;
            }
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(CommitInfo::from_git2_commit(&commit));
        }
        Ok(commits)
    }

    /// Retourne la liste des commits depuis toutes les branches.
    pub fn log_all_branches(&self, max_count: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;

        // Pousser toutes les refs locales (branches, tags)
        for reference in self.repo.references()? {
            let reference = reference?;
            if let Some(oid) = reference.target() {
                revwalk.push(oid).ok();
            }
        }

        revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;

        let mut commits = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if i >= max_count {
                break;
            }
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(CommitInfo::from_git2_commit(&commit));
        }
        Ok(commits)
    }

    /// Construit le graphe de commits pour l'affichage.
    pub fn build_graph(&self, max_count: usize) -> Result<Vec<GraphRow>> {
        let commits = self.log_all_branches(max_count)?;
        let graph = super::graph::build_graph(&self.repo, &commits)?;
        Ok(graph)
    }

    /// Retourne le status du working directory.
    pub fn status(&self) -> Result<Vec<StatusEntry>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("???").to_string();
            let status = entry.status();
            entries.push(StatusEntry { path, status });
        }
        Ok(entries)
    }

    /// Retourne la liste des branches locales.
    pub fn branches(&self) -> Result<Vec<BranchInfo>> {
        super::branch::list_branches(&self.repo)
    }

    /// Retourne la liste des stashes.
    pub fn stashes(&mut self) -> Result<Vec<StashEntry>> {
        super::stash::list_stashes(&mut self.repo)
    }

    /// Retourne le diff d'un fichier dans un stash.
    pub fn stash_file_diff(&self, stash_oid: git2::Oid, file_path: &str) -> Result<Vec<String>> {
        super::stash::stash_file_diff(&self.repo, stash_oid, file_path)
    }

    /// Retourne le diff d'un commit.
    pub fn commit_diff(&self, oid: git2::Oid) -> Result<Vec<super::diff::DiffFile>> {
        super::diff::commit_diff(&self.repo, oid)
    }

    /// Retourne le diff détaillé d'un fichier spécifique dans un commit.
    pub fn file_diff(&self, oid: git2::Oid, file_path: &str) -> Result<super::diff::FileDiff> {
        super::diff::get_file_diff(&self.repo, oid, file_path)
    }

    /// Checkout une branche existante.
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        super::branch::checkout_branch(&self.repo, name)
    }

    /// Retourne la liste des worktrees.
    pub fn worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        super::worktree::list_worktrees(&self.repo)
    }

    /// Crée un nouveau worktree.
    pub fn create_worktree(&self, name: &str, path: &str, branch: Option<&str>) -> Result<()> {
        super::worktree::create_worktree(&self.repo, name, path, branch)
    }

    /// Supprime un worktree.
    pub fn remove_worktree(&self, name: &str) -> Result<()> {
        super::worktree::remove_worktree(&self.repo, name)
    }
}

/// Entrée de status (fichier + état).
#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub status: git2::Status,
}

impl StatusEntry {
    /// Retourne une description lisible du status.
    pub fn display_status(&self) -> &'static str {
        let s = self.status;
        if s.contains(git2::Status::INDEX_NEW) {
            "Nouveau (staged)"
        } else if s.contains(git2::Status::INDEX_MODIFIED) {
            "Modifié (staged)"
        } else if s.contains(git2::Status::INDEX_DELETED) {
            "Supprimé (staged)"
        } else if s.contains(git2::Status::WT_MODIFIED) {
            "Modifié"
        } else if s.contains(git2::Status::WT_NEW) {
            "Non suivi"
        } else if s.contains(git2::Status::WT_DELETED) {
            "Supprimé"
        } else {
            "Inconnu"
        }
    }

    /// Retourne true si le fichier est staged (dans l'index).
    pub fn is_staged(&self) -> bool {
        self.status.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED,
        )
    }

    /// Retourne true si le fichier est unstaged (dans le working directory).
    pub fn is_unstaged(&self) -> bool {
        self.status.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_NEW
                | git2::Status::WT_RENAMED,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;

    #[test]
    fn test_status_entry_is_staged() {
        // INDEX_NEW - fichier nouveau staged
        let entry_new = StatusEntry {
            path: "new.txt".to_string(),
            status: git2::Status::INDEX_NEW,
        };
        assert!(entry_new.is_staged());
        assert!(!entry_new.is_unstaged());

        // INDEX_MODIFIED - fichier modifié staged
        let entry_modified = StatusEntry {
            path: "modified.txt".to_string(),
            status: git2::Status::INDEX_MODIFIED,
        };
        assert!(entry_modified.is_staged());
        assert!(!entry_modified.is_unstaged());

        // WT_MODIFIED - fichier modifié non staged
        let entry_wt = StatusEntry {
            path: "wt_modified.txt".to_string(),
            status: git2::Status::WT_MODIFIED,
        };
        assert!(!entry_wt.is_staged());
        assert!(entry_wt.is_unstaged());

        // WT_NEW - fichier non suivi
        let entry_untracked = StatusEntry {
            path: "untracked.txt".to_string(),
            status: git2::Status::WT_NEW,
        };
        assert!(!entry_untracked.is_staged());
        assert!(entry_untracked.is_unstaged());
    }

    #[test]
    fn test_status_entry_display_status() {
        let entry_new = StatusEntry {
            path: "new.txt".to_string(),
            status: git2::Status::INDEX_NEW,
        };
        assert_eq!(entry_new.display_status(), "Nouveau (staged)");

        let entry_modified = StatusEntry {
            path: "modified.txt".to_string(),
            status: git2::Status::WT_MODIFIED,
        };
        assert_eq!(entry_modified.display_status(), "Modifié");

        let entry_untracked = StatusEntry {
            path: "untracked.txt".to_string(),
            status: git2::Status::WT_NEW,
        };
        assert_eq!(entry_untracked.display_status(), "Non suivi");

        let entry_deleted = StatusEntry {
            path: "deleted.txt".to_string(),
            status: git2::Status::WT_DELETED,
        };
        assert_eq!(entry_deleted.display_status(), "Supprimé");
    }

    #[test]
    fn test_git_repo_open() {
        let (_temp_dir, repo) = create_test_repo();
        let path = repo.workdir().unwrap().to_str().unwrap();

        let git_repo = GitRepo::open(path).unwrap();
        // Vérifier que le repo est bien ouvert
        assert!(git_repo.repo.workdir().is_some());
    }

    #[test]
    fn test_git_repo_current_branch() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un premier commit pour avoir une branche
        commit_file(&repo, "test.txt", "content", "Initial commit");

        let git_repo = GitRepo::open(repo.workdir().unwrap().to_str().unwrap()).unwrap();
        let branch = git_repo.current_branch().unwrap();

        // La branche devrait s'appeler "main"
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_git_repo_log() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer plusieurs commits
        commit_file(&repo, "test.txt", "A", "First commit");
        commit_file(&repo, "test.txt", "B", "Second commit");
        commit_file(&repo, "test.txt", "C", "Third commit");

        let git_repo = GitRepo::open(repo.workdir().unwrap().to_str().unwrap()).unwrap();
        let commits = git_repo.log(10).unwrap();

        // Devrait avoir 3 commits
        assert_eq!(commits.len(), 3);
        // Le premier commit dans le log est le plus récent
        assert_eq!(commits[0].message, "Third commit");
        assert_eq!(commits[1].message, "Second commit");
        assert_eq!(commits[2].message, "First commit");
    }

    #[test]
    fn test_git_repo_status() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "content", "Initial commit");

        let git_repo = GitRepo::open(repo.workdir().unwrap().to_str().unwrap()).unwrap();

        // Modifier un fichier
        create_file(&repo, "test.txt", "modified content");

        let status = git_repo.status().unwrap();
        // Devrait avoir 1 fichier modifié
        assert_eq!(status.len(), 1);
        assert!(status[0].is_unstaged());
        assert!(!status[0].is_staged());
    }

    #[test]
    fn test_git_repo_branches() {
        let (_temp_dir, repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "content", "Initial commit");

        // Créer une branche
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &head, false).unwrap();

        let git_repo = GitRepo::open(repo.workdir().unwrap().to_str().unwrap()).unwrap();
        let branches = git_repo.branches().unwrap();

        // Devrait avoir 2 branches: main et feature
        assert_eq!(branches.len(), 2);
    }
}
