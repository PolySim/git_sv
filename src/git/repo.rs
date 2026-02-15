use git2::{Repository, StatusOptions};

use super::branch::BranchInfo;
use super::commit::CommitInfo;
use super::graph::GraphRow;
use super::stash::StashEntry;
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
