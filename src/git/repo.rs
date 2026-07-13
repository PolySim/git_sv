//! Wrapper `GitRepo` autour de `git2::Repository`.
//!
//! Fournit les opérations de haut niveau : log, statut, diffs,
//! construction du graphe, et accès aux données du repository.

#![allow(dead_code)]

use crate::i18n::text;
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
        let name = head
            .shorthand()
            .unwrap_or(text("HEAD detachee", "Detached HEAD"))
            .to_string();
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
        self.log_all_branches_offset(0, max_count)
    }

    /// Retourne la liste des commits depuis toutes les branches avec offset.
    ///
    /// Cette méthode permet le chargement progressif en chargeant les commits
    /// à partir d'un offset donné.
    ///
    /// # Arguments
    /// * `skip` - Nombre de commits à sauter (offset)
    /// * `max_count` - Nombre maximum de commits à retourner
    pub fn log_all_branches_offset(
        &self,
        skip: usize,
        max_count: usize,
    ) -> Result<Vec<CommitInfo>> {
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
            if i < skip {
                continue;
            }
            if i >= skip + max_count {
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
        self.build_graph_offset(0, max_count)
    }

    /// Construit le graphe de commits et indique s'il reste de l'historique à charger.
    pub fn build_graph_with_more(&self, max_count: usize) -> Result<(Vec<GraphRow>, bool)> {
        self.build_graph_offset_with_more(0, max_count)
    }

    /// Construit le graphe de commits à partir d'un offset.
    ///
    /// Cette méthode permet le chargement progressif en chargeant les commits
    /// à partir d'un offset donné.
    pub fn build_graph_offset(&self, skip: usize, max_count: usize) -> Result<Vec<GraphRow>> {
        let (graph, _) = self.build_graph_offset_with_more(skip, max_count)?;
        Ok(graph)
    }

    /// Construit le graphe de commits à partir d'un offset et indique s'il reste
    /// potentiellement d'autres commits à charger.
    pub fn build_graph_offset_with_more(
        &self,
        skip: usize,
        max_count: usize,
    ) -> Result<(Vec<GraphRow>, bool)> {
        let fetch_count = max_count.saturating_add(1);
        let commits = self.log_all_branches_offset(skip, fetch_count)?;
        let has_more = commits.len() > max_count;
        let commits = commits.into_iter().take(max_count).collect::<Vec<_>>();
        let graph = super::graph::build_graph(&self.repo, &commits)?;
        Ok((graph, has_more))
    }

    /// Estime le nombre total de commits dans le repository.
    ///
    /// Retourne None si l'estimation échoue.
    pub fn estimate_total_commits(&self) -> Option<usize> {
        // Utiliser une limite élevée pour obtenir une estimation
        let mut revwalk = self.repo.revwalk().ok()?;

        // Pousser toutes les refs
        for reference in self.repo.references().ok()?.flatten() {
            if let Some(oid) = reference.target() {
                revwalk.push(oid).ok()?;
            }
        }

        revwalk
            .set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)
            .ok()?;

        // Compter les commits (avec une limite de sécurité)
        let max_to_count = crate::state::MAX_TOTAL_COMMITS;
        let count = revwalk.take(max_to_count).count();

        Some(count)
    }

    /// Construit le graphe de commits avec filtrage.
    pub fn build_graph_filtered(
        &self,
        max_count: usize,
        filter: &crate::state::GraphFilter,
    ) -> Result<Vec<GraphRow>> {
        let (graph, _) = self.build_graph_filtered_with_more(max_count, filter)?;
        Ok(graph)
    }

    /// Construit le graphe de commits avec filtrage et indique s'il reste
    /// potentiellement d'autres résultats à charger.
    pub fn build_graph_filtered_with_more(
        &self,
        max_count: usize,
        filter: &crate::state::GraphFilter,
    ) -> Result<(Vec<GraphRow>, bool)> {
        let (commits, has_more) = self.log_filtered_with_more(max_count, filter)?;
        let graph = super::graph::build_graph(&self.repo, &commits)?;
        Ok((graph, has_more))
    }

    /// Retourne le log filtré des commits.
    pub fn log_filtered(
        &self,
        max_count: usize,
        filter: &crate::state::GraphFilter,
    ) -> Result<Vec<CommitInfo>> {
        let (commits, _) = self.log_filtered_with_more(max_count, filter)?;
        Ok(commits)
    }

    /// Retourne le log filtré des commits avec indication de résultats supplémentaires.
    pub fn log_filtered_with_more(
        &self,
        max_count: usize,
        filter: &crate::state::GraphFilter,
    ) -> Result<(Vec<CommitInfo>, bool)> {
        let chunk_size = max_count.max(crate::state::COMMIT_BATCH_SIZE).max(1);
        let mut skip = 0;
        let mut results = Vec::with_capacity(max_count.saturating_add(1));
        let mut has_more = false;

        loop {
            let mut commits = self.log_all_branches_offset(skip, chunk_size)?;

            if commits.is_empty() {
                break;
            }

            if filter.path.is_some() {
                for commit in &mut commits {
                    if commit.changed_paths.is_none() {
                        if let Err(e) = commit.load_changed_paths(&self.repo) {
                            eprintln!("Erreur chargement chemins pour {}: {}", commit.oid, e);
                        }
                    }
                }
            }

            for commit in filter.filter_commits(&commits) {
                results.push(commit);
                if results.len() > max_count {
                    has_more = true;
                    results.truncate(max_count);
                    break;
                }
            }

            if has_more {
                break;
            }

            let fetched_len = commits.len();
            skip += fetched_len;

            if fetched_len < chunk_size {
                break;
            }
        }

        Ok((results, has_more))
    }

    /// Recherche des commits par message ou auteur.
    pub fn search_commits(&self, query: &str, max_count: usize) -> Result<Vec<CommitInfo>> {
        let query_lower = query.to_lowercase();
        let commits = self.log_all_branches(max_count * 3)?;

        let filtered: Vec<CommitInfo> = commits
            .into_iter()
            .filter(|c| {
                c.message.to_lowercase().contains(&query_lower)
                    || c.author.to_lowercase().contains(&query_lower)
                    || c.oid.to_string().to_lowercase().starts_with(&query_lower)
            })
            .take(max_count)
            .collect();

        Ok(filtered)
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

    /// Retourne les fichiers présents dans le worktree courant.
    pub fn current_project_files(&self) -> Result<Vec<String>> {
        super::project_tree::current_project_files(&self.repo)
    }

    /// Retourne l'historique des modifications d'un fichier ou dossier.
    pub fn path_history(
        &self,
        path: &str,
        is_directory: bool,
        max_count: usize,
    ) -> Result<Vec<CommitInfo>> {
        super::project_tree::path_history(&self.repo, path, is_directory, max_count)
    }

    /// Lit le contenu texte d'un fichier tel qu'il existe dans un commit.
    pub fn file_content_at_commit(&self, oid: git2::Oid, path: &str) -> Result<Option<String>> {
        super::project_tree::file_content_at_commit(&self.repo, oid, path)
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

/// Type de status d'un fichier dans le working directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatusKind {
    /// Fichier modifié.
    Modified,
    /// Fichier supprimé.
    Deleted,
    /// Fichier non suivi.
    Untracked,
    /// Fichier renommé.
    Renamed,
    /// Fichier indexé.
    Staged,
    /// Nouveau fichier indexé.
    NewStaged,
    /// Fichier en conflit.
    Conflicted,
}

impl Default for StatusEntry {
    fn default() -> Self {
        Self {
            path: String::new(),
            status: git2::Status::empty(),
        }
    }
}

impl StatusEntry {
    /// Retourne le type de status sous forme d'enum.
    pub fn status_kind(&self) -> FileStatusKind {
        let status = self.status;

        if status.is_conflicted() {
            FileStatusKind::Conflicted
        } else if status.is_index_new() {
            FileStatusKind::NewStaged
        } else if status.is_index_modified()
            || status.is_index_deleted()
            || status.is_index_renamed()
        {
            FileStatusKind::Staged
        } else if status.is_wt_modified() {
            FileStatusKind::Modified
        } else if status.is_wt_deleted() {
            FileStatusKind::Deleted
        } else if status.is_wt_new() {
            FileStatusKind::Untracked
        } else if status.is_wt_renamed() {
            FileStatusKind::Renamed
        } else {
            FileStatusKind::Modified
        }
    }

    /// Retourne une description lisible du status.
    pub fn display_status(&self) -> &'static str {
        match self.status_kind() {
            FileStatusKind::Modified => text("Modifié", "Modified"),
            FileStatusKind::Deleted => text("Supprimé", "Deleted"),
            FileStatusKind::Untracked => text("Non suivi", "Untracked"),
            FileStatusKind::Renamed => text("Renommé", "Renamed"),
            FileStatusKind::Staged => text("Indexé", "Staged"),
            FileStatusKind::NewStaged => text("Nouveau (staged)", "New (staged)"),
            FileStatusKind::Conflicted => text("Conflit", "Conflicted"),
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
    use crate::i18n::{with_language, Language};

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
        with_language(Language::Fr, || {
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

            let entry_staged = StatusEntry {
                path: "staged.txt".to_string(),
                status: git2::Status::INDEX_MODIFIED,
            };
            assert_eq!(entry_staged.display_status(), "Indexé");

            let entry_renamed = StatusEntry {
                path: "renamed.txt".to_string(),
                status: git2::Status::WT_RENAMED,
            };
            assert_eq!(entry_renamed.display_status(), "Renommé");

            let entry_conflicted = StatusEntry {
                path: "conflicted.txt".to_string(),
                status: git2::Status::CONFLICTED,
            };
            assert_eq!(entry_conflicted.display_status(), "Conflit");
        });
    }

    #[test]
    fn test_status_entry_status_kind() {
        let cases = [
            (git2::Status::INDEX_NEW, FileStatusKind::NewStaged),
            (git2::Status::INDEX_MODIFIED, FileStatusKind::Staged),
            (git2::Status::INDEX_DELETED, FileStatusKind::Staged),
            (git2::Status::INDEX_RENAMED, FileStatusKind::Staged),
            (git2::Status::WT_MODIFIED, FileStatusKind::Modified),
            (git2::Status::WT_DELETED, FileStatusKind::Deleted),
            (git2::Status::WT_NEW, FileStatusKind::Untracked),
            (git2::Status::WT_RENAMED, FileStatusKind::Renamed),
            (git2::Status::CONFLICTED, FileStatusKind::Conflicted),
        ];

        for (status, expected) in cases {
            let entry = StatusEntry {
                path: "file.txt".to_string(),
                status,
            };

            assert_eq!(entry.status_kind(), expected);
        }
    }

    #[test]
    fn test_status_entry_status_kind_prioritizes_index_over_worktree() {
        let entry = StatusEntry {
            path: "file.txt".to_string(),
            status: git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED,
        };

        assert_eq!(entry.status_kind(), FileStatusKind::Staged);
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
