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
        self.all_commits_revwalk()?
            .skip(skip)
            .take(max_count)
            .map(|oid| {
                let commit = self.repo.find_commit(oid?)?;
                Ok(CommitInfo::from_git2_commit(&commit))
            })
            .collect()
    }

    /// Inclut HEAD détachée et les tags annotés pointant vers des commits.
    fn all_commits_revwalk(&self) -> Result<git2::Revwalk<'_>> {
        let mut revwalk = self.repo.revwalk()?;
        match revwalk.push_head() {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.code(),
                    git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
                ) => {}
            Err(error) => return Err(error.into()),
        }
        for reference in self.repo.references()? {
            let reference = reference?;
            if let Ok(commit) = reference.peel_to_commit() {
                revwalk.push(commit.id())?;
            }
        }
        revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;
        Ok(revwalk)
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
        self.all_commits_revwalk()
            .ok()?
            .take(crate::state::MAX_TOTAL_COMMITS)
            .collect::<std::result::Result<Vec<_>, _>>()
            .ok()
            .map(|commits| commits.len())
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
        let mut results = Vec::new();
        let mut metadata_filter = filter.clone();
        metadata_filter.path = None;
        for oid in self.all_commits_revwalk()? {
            let raw_commit = self.repo.find_commit(oid?)?;
            let mut commit = CommitInfo::from_git2_commit(&raw_commit);
            if filter.path.is_some() {
                // Éviter de lire les arbres d'un commit déjà exclu par ses métadonnées.
                if !metadata_filter.matches(&commit) {
                    continue;
                }
                commit.load_changed_paths(&self.repo)?;
            }
            if filter.matches(&commit) {
                if results.len() == max_count {
                    return Ok((results, true));
                }
                results.push(commit);
            }
        }
        Ok((results, false))
    }

    /// Recherche des commits par message ou auteur.
    pub fn search_commits(&self, query: &str, max_count: usize) -> Result<Vec<CommitInfo>> {
        if max_count == 0 {
            return Ok(Vec::new());
        }
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        for oid in self.all_commits_revwalk()? {
            let raw_commit = self.repo.find_commit(oid?)?;
            let commit = CommitInfo::from_git2_commit(&raw_commit);
            if commit.message.to_lowercase().contains(&query_lower)
                || commit.author.to_lowercase().contains(&query_lower)
                || commit.oid.to_string().starts_with(&query_lower)
            {
                results.push(commit);
                if results.len() == max_count {
                    break;
                }
            }
        }
        Ok(results)
    }

    /// Retourne le status du working directory.
    pub fn status(&self) -> Result<Vec<StatusEntry>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let path = entry
                .path()
                .map_err(|_| crate::error::GitSvError::UnsupportedPathEncoding)?
                .to_string();
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

    /// Compare l'historique d'un chemin entre HEAD et une branche.
    pub fn compare_path_history(
        &self,
        path: &str,
        is_directory: bool,
        target_branch: &str,
        max_count: usize,
    ) -> Result<super::project_tree::PathHistoryComparison> {
        super::project_tree::compare_path_history(
            &self.repo,
            path,
            is_directory,
            target_branch,
            max_count,
        )
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
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE,
        )
    }

    /// Retourne true si le fichier est unstaged (dans le working directory).
    pub fn is_unstaged(&self) -> bool {
        self.status.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_NEW
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn safety_status_rejects_non_utf8_without_aliasing_a_real_file() {
        let (temp, repo) = crate::git::tests::test_utils::create_test_repo();
        crate::git::tests::test_utils::commit_file(&repo, "???", "unrelated", "initial");
        crate::git::tests::test_utils::add_raw_index_file(&repo, b"bad\xff.txt", b"raw name");
        let error = GitRepo { repo }.status().unwrap_err();
        assert!(matches!(
            error,
            crate::error::GitSvError::UnsupportedPathEncoding
        ));
        assert_eq!(
            std::fs::read(temp.path().join("???")).unwrap(),
            b"unrelated"
        );
    }
    use crate::git::tests::test_utils::*;
    use crate::i18n::{with_language, Language};

    #[test]
    #[ignore = "profilage en lecture seule via GIT_SV_PROFILE_REPO"]
    fn profile_large_repository_history() {
        use std::time::Instant;

        let path = std::env::var("GIT_SV_PROFILE_REPO").expect("GIT_SV_PROFILE_REPO requis");
        for pass in 0..3 {
            let repo = GitRepo::open(&path).unwrap();
            let start = Instant::now();
            let walk = repo.all_commits_revwalk().unwrap();
            let setup = start.elapsed();
            let start = Instant::now();
            let ids = walk
                .take(201)
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap();
            let walk_time = start.elapsed();
            let start = Instant::now();
            let commits = ids
                .iter()
                .map(|&oid| CommitInfo::from_git2_commit(&repo.repo.find_commit(oid).unwrap()))
                .collect::<Vec<_>>();
            let decode = start.elapsed();
            let start = Instant::now();
            let graph = super::super::graph::build_graph(&repo.repo, &commits).unwrap();
            let layout = start.elapsed();
            let start = Instant::now();
            let count = repo.estimate_total_commits();
            let count_time = start.elapsed();
            let start = Instant::now();
            let files = repo.commit_diff(ids[0]).unwrap();
            let list_time = start.elapsed();
            let start = Instant::now();
            if let Some(file) = files.first() {
                let _ = repo.file_diff(ids[0], &file.path).unwrap();
            }
            let diff_time = start.elapsed();
            eprintln!("history pass={pass} refs={setup:?} walk={walk_time:?} decode={decode:?} layout={layout:?} count={count:?}/{count_time:?} files={}/{list_time:?} diff={diff_time:?} rows={}", files.len(), graph.len());
            if pass == 0 {
                let start = Instant::now();
                let file_count = ids
                    .iter()
                    .take(20)
                    .map(|&oid| repo.commit_diff(oid).unwrap().len())
                    .sum::<usize>();
                let stats_time = start.elapsed();
                let start = Instant::now();
                let paths_count = ids
                    .iter()
                    .take(20)
                    .map(|&oid| {
                        super::super::diff::commit_changed_paths(&repo.repo, oid)
                            .unwrap()
                            .len()
                    })
                    .sum::<usize>();
                eprintln!("paths sample=20 files={file_count}/{paths_count} with_stats={stats_time:?} without_stats={:?}", start.elapsed());
                assert_eq!(file_count, paths_count);
            }
        }
    }

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

    #[test]
    fn history_includes_detached_head_without_a_reference() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "file", "base", "base");
        let branch = repo.head().unwrap().name().unwrap().to_string();
        let tip = commit_file(&repo, "file", "tip", "detached");
        repo.set_head_detached(tip).unwrap();
        repo.find_reference(&branch).unwrap().delete().unwrap();
        let wrapper = GitRepo { repo };
        assert_eq!(wrapper.log_all_branches(10).unwrap()[0].oid, tip);
    }

    #[test]
    fn search_finds_old_commits_beyond_three_times_the_requested_count() {
        let (_temp, repo) = create_test_repo();
        let expected = commit_file(&repo, "file", "0", "needle");
        for index in 1..8 {
            commit_file(&repo, "file", &index.to_string(), "unrelated");
        }
        let wrapper = GitRepo { repo };
        assert_eq!(
            wrapper.search_commits("needle", 1).unwrap()[0].oid,
            expected
        );
    }

    #[test]
    fn path_and_message_filters_preserve_order_and_pagination() {
        let (_temp, repo) = create_test_repo();
        let first = commit_file(&repo, "src/keep.txt", "first", "selected first");
        commit_file(&repo, "docs/readme.txt", "other path", "selected unrelated");
        commit_file(&repo, "src/keep.txt", "ignored", "unrelated message");
        let last = commit_file(&repo, "src/keep.txt", "last", "selected last");
        let wrapper = GitRepo { repo };
        let filter = crate::state::GraphFilter {
            path: Some("SRC/KEEP".to_string()),
            message: Some("selected".to_string()),
            ..Default::default()
        };

        let (commits, has_more) = wrapper.log_filtered_with_more(1, &filter).unwrap();
        assert_eq!(
            commits.iter().map(|commit| commit.oid).collect::<Vec<_>>(),
            vec![last]
        );
        assert!(has_more);
        let (commits, has_more) = wrapper.log_filtered_with_more(2, &filter).unwrap();
        assert_eq!(
            commits.iter().map(|commit| commit.oid).collect::<Vec<_>>(),
            vec![last, first]
        );
        assert!(!has_more);
        assert_eq!(
            commits[0].changed_paths.as_deref(),
            Some(["src/keep.txt".to_string()].as_slice())
        );
    }

    #[test]
    fn large_limits_do_not_preallocate_or_overflow() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "file", "base", "base");
        let wrapper = GitRepo { repo };
        assert_eq!(
            wrapper
                .log_filtered(usize::MAX, &Default::default())
                .unwrap()
                .len(),
            1
        );
        assert_eq!(wrapper.search_commits("base", usize::MAX).unwrap().len(), 1);
        assert!(wrapper
            .log_all_branches_offset(usize::MAX, 10)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn type_changes_are_visible_in_staging_lists() {
        let entry = StatusEntry {
            path: "file".to_string(),
            status: git2::Status::INDEX_TYPECHANGE | git2::Status::WT_TYPECHANGE,
        };
        assert!(entry.is_staged());
        assert!(entry.is_unstaged());
    }
}
