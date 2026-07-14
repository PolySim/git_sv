//! Accès à l'arborescence courante et à l'historique par chemin.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::Path;

use git2::{Delta, ErrorCode, Oid, Repository, Sort, Tree};

use crate::error::Result;

use super::commit::CommitInfo;

/// Côté auquel appartient un commit divergent d'historique de chemin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathHistorySide {
    Current,
    Target,
}

/// Commit d'un historique de chemin comparé, annoté par branche.
#[derive(Debug, Clone)]
pub struct ComparedPathCommit {
    pub commit: CommitInfo,
    pub side: PathHistorySide,
}

/// Historique divergent d'un chemin entre HEAD et une branche.
#[derive(Debug, Clone)]
pub struct PathHistoryComparison {
    pub commits: Vec<ComparedPathCommit>,
    pub ahead: usize,
    pub behind: usize,
}

/// Retourne les chemins de fichiers présents dans le worktree courant.
///
/// La liste combine l'index et les fichiers non suivis remontés par Git. Les
/// fichiers ignorés et le répertoire `.git` ne sont donc pas inclus.
pub fn current_project_files(repo: &Repository) -> Result<Vec<String>> {
    let Some(workdir) = repo.workdir() else {
        return Ok(Vec::new());
    };

    let mut files = BTreeSet::new();
    let index = repo.index()?;
    for entry in index.iter() {
        let path = String::from_utf8_lossy(&entry.path).into_owned();
        if std::fs::symlink_metadata(workdir.join(&path)).is_ok() {
            files.insert(path);
        }
    }

    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    for entry in repo.statuses(Some(&mut options))?.iter() {
        let Some(path) = entry.path() else {
            continue;
        };
        if entry.status().intersects(
            git2::Status::WT_NEW
                | git2::Status::INDEX_NEW
                | git2::Status::WT_MODIFIED
                | git2::Status::INDEX_MODIFIED,
        ) && std::fs::symlink_metadata(workdir.join(path)).is_ok()
        {
            files.insert(path.to_string());
        }
    }

    Ok(files.into_iter().collect())
}

/// Retourne les commits de `HEAD` qui ont modifié un fichier ou un dossier.
///
/// Pour un fichier, les renommages sont suivis vers son ancien chemin afin de
/// conserver l'historique antérieur au renommage.
pub fn path_history(
    repo: &Repository,
    path: &str,
    is_directory: bool,
    max_count: usize,
) -> Result<Vec<CommitInfo>> {
    if path.is_empty() || max_count == 0 {
        return Ok(Vec::new());
    }

    let head = repo.head()?.peel_to_commit()?;
    path_history_from(repo, head.id(), None, path, is_directory, max_count)
}

/// Retourne les commits divergents qui ont modifié un chemin sur deux branches.
pub fn compare_path_history(
    repo: &Repository,
    path: &str,
    is_directory: bool,
    target_branch: &str,
    max_count: usize,
) -> Result<PathHistoryComparison> {
    if path.is_empty() || max_count == 0 {
        return Ok(PathHistoryComparison {
            commits: Vec::new(),
            ahead: 0,
            behind: 0,
        });
    }

    let head = repo.head()?.peel_to_commit()?;
    let target = repo.revparse_single(target_branch)?.peel_to_commit()?;
    let target_path = match repo.merge_base(head.id(), target.id()) {
        Ok(merge_base) => {
            let common_path = path_at_ancestor(repo, head.id(), merge_base, path, is_directory)?;
            path_at_descendant(repo, target.id(), merge_base, &common_path, is_directory)?
        }
        Err(_) => path.to_string(),
    };
    let current_history = path_history_from(
        repo,
        head.id(),
        Some(target.id()),
        path,
        is_directory,
        max_count,
    )?;
    let target_history = path_history_from(
        repo,
        target.id(),
        Some(head.id()),
        &target_path,
        is_directory,
        max_count,
    )?;
    let ahead = current_history.len();
    let behind = target_history.len();

    let current_commits = current_history
        .into_iter()
        .map(|commit| ComparedPathCommit {
            commit,
            side: PathHistorySide::Current,
        })
        .collect();
    let target_commits = target_history
        .into_iter()
        .map(|commit| ComparedPathCommit {
            commit,
            side: PathHistorySide::Target,
        })
        .collect();
    let commits = merge_compared_histories(current_commits, target_commits, max_count);

    Ok(PathHistoryComparison {
        commits,
        ahead,
        behind,
    })
}

fn path_history_from(
    repo: &Repository,
    start_oid: Oid,
    hidden_oid: Option<Oid>,
    path: &str,
    is_directory: bool,
    max_count: usize,
) -> Result<Vec<CommitInfo>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push(start_oid)?;
    if let Some(hidden_oid) = hidden_oid {
        revwalk.hide(hidden_oid)?;
    }
    revwalk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;

    let mut tracked_paths = HashMap::<Oid, BTreeSet<String>>::new();
    tracked_paths.insert(start_oid, BTreeSet::from([path.to_string()]));
    let mut history = Vec::new();

    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;
        let Some(paths) = tracked_paths.remove(&commit.id()) else {
            continue;
        };
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let mut matched = false;
        for tracked_path in &paths {
            let current_entry = tree_entry_identity(Some(&tree), tracked_path)?;
            let parent_entry = tree_entry_identity(parent_tree.as_ref(), tracked_path)?;
            if current_entry != parent_entry {
                matched = true;
                break;
            }
        }

        if matched {
            history.push(CommitInfo::from_git2_commit(&commit));
            if history.len() >= max_count {
                break;
            }
        }

        for parent in commit.parents() {
            let parent_tree = parent.tree()?;
            let parent_paths = tracked_paths.entry(parent.id()).or_default();
            for tracked_path in &paths {
                parent_paths.insert(previous_path(
                    repo,
                    &parent_tree,
                    &tree,
                    tracked_path,
                    is_directory,
                )?);
            }
        }
    }

    Ok(history)
}

fn merge_compared_histories(
    current: Vec<ComparedPathCommit>,
    target: Vec<ComparedPathCommit>,
    max_count: usize,
) -> Vec<ComparedPathCommit> {
    let mut current = VecDeque::from(current);
    let mut target = VecDeque::from(target);
    let mut merged = Vec::with_capacity(max_count.min(current.len() + target.len()));

    while merged.len() < max_count && (!current.is_empty() || !target.is_empty()) {
        let take_current = match (current.front(), target.front()) {
            (Some(current), Some(target)) => current.commit.timestamp >= target.commit.timestamp,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if take_current {
            if let Some(commit) = current.pop_front() {
                merged.push(commit);
            }
        } else if let Some(commit) = target.pop_front() {
            merged.push(commit);
        }
    }

    merged
}

fn path_at_ancestor(
    repo: &Repository,
    descendant: Oid,
    ancestor: Oid,
    path: &str,
    is_directory: bool,
) -> Result<String> {
    let chain = ancestry_chain(repo, descendant, ancestor)?;
    let mut tracked_path = path.to_string();

    for pair in chain.windows(2) {
        let commit_tree = repo.find_commit(pair[0])?.tree()?;
        let parent_tree = repo.find_commit(pair[1])?.tree()?;
        tracked_path = previous_path(
            repo,
            &parent_tree,
            &commit_tree,
            &tracked_path,
            is_directory,
        )?;
    }

    Ok(tracked_path)
}

fn path_at_descendant(
    repo: &Repository,
    descendant: Oid,
    ancestor: Oid,
    path: &str,
    is_directory: bool,
) -> Result<String> {
    let chain = ancestry_chain(repo, descendant, ancestor)?;
    let mut tracked_path = path.to_string();

    for pair in chain.windows(2).rev() {
        let commit_tree = repo.find_commit(pair[0])?.tree()?;
        let parent_tree = repo.find_commit(pair[1])?.tree()?;
        if !is_directory
            && tree_entry_identity(Some(&parent_tree), &tracked_path)?.is_some()
            && tree_entry_identity(Some(&commit_tree), &tracked_path)?.is_none()
        {
            if let Some(next_path) =
                renamed_to_path(repo, &parent_tree, &commit_tree, &tracked_path)?
            {
                tracked_path = next_path;
            }
        }
    }

    Ok(tracked_path)
}

fn ancestry_chain(repo: &Repository, descendant: Oid, ancestor: Oid) -> Result<Vec<Oid>> {
    let mut chain = vec![descendant];
    let mut current = descendant;

    while current != ancestor {
        let commit = repo.find_commit(current)?;
        let mut next = None;
        for parent in commit.parents() {
            if parent.id() == ancestor || repo.graph_descendant_of(parent.id(), ancestor)? {
                next = Some(parent.id());
                break;
            }
        }
        let Some(parent) = next else {
            return Err(git2::Error::from_str("ancetre inaccessible depuis le commit").into());
        };
        chain.push(parent);
        current = parent;
    }

    Ok(chain)
}

fn previous_path(
    repo: &Repository,
    parent_tree: &Tree<'_>,
    tree: &Tree<'_>,
    path: &str,
    is_directory: bool,
) -> Result<String> {
    if !is_directory
        && tree_entry_identity(Some(tree), path)?.is_some()
        && tree_entry_identity(Some(parent_tree), path)?.is_none()
    {
        if let Some(previous_path) = renamed_from_path(repo, Some(parent_tree), tree, path)? {
            return Ok(previous_path);
        }
    }

    Ok(path.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TreeEntryIdentity {
    oid: Oid,
    filemode: i32,
}

fn tree_entry_identity(tree: Option<&Tree<'_>>, path: &str) -> Result<Option<TreeEntryIdentity>> {
    let Some(tree) = tree else {
        return Ok(None);
    };

    match tree.get_path(Path::new(path)) {
        Ok(entry) => Ok(Some(TreeEntryIdentity {
            oid: entry.id(),
            filemode: entry.filemode_raw(),
        })),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn renamed_from_path(
    repo: &Repository,
    parent_tree: Option<&Tree<'_>>,
    tree: &Tree<'_>,
    path: &str,
) -> Result<Option<String>> {
    let mut diff = repo.diff_tree_to_tree(parent_tree, Some(tree), None)?;
    diff.find_similar(None)?;

    Ok(diff.deltas().find_map(|delta| {
        (delta.status() == Delta::Renamed && path_matches(delta.new_file().path(), path, false))
            .then(|| {
                delta
                    .old_file()
                    .path()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .flatten()
    }))
}

fn renamed_to_path(
    repo: &Repository,
    parent_tree: &Tree<'_>,
    tree: &Tree<'_>,
    path: &str,
) -> Result<Option<String>> {
    let mut diff = repo.diff_tree_to_tree(Some(parent_tree), Some(tree), None)?;
    diff.find_similar(None)?;

    Ok(diff.deltas().find_map(|delta| {
        (delta.status() == Delta::Renamed && path_matches(delta.old_file().path(), path, false))
            .then(|| {
                delta
                    .new_file()
                    .path()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .flatten()
    }))
}

/// Lit le contenu texte d'un fichier tel qu'il existe dans un commit.
///
/// Retourne `None` lorsque le chemin n'existe pas dans l'arbre du commit.
pub fn file_content_at_commit(
    repo: &Repository,
    oid: git2::Oid,
    path: &str,
) -> Result<Option<String>> {
    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;
    let entry = match tree.get_path(Path::new(path)) {
        Ok(entry) => entry,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let object = entry.to_object(repo)?;
    let blob = object.as_blob().ok_or_else(|| {
        crate::error::GitSvError::Other(format!("Le chemin '{path}' n'est pas un fichier"))
    })?;
    let content = std::str::from_utf8(blob.content())
        .map_err(|_| crate::error::GitSvError::Other(format!("Le fichier '{path}' est binaire")))?;
    Ok(Some(content.to_string()))
}

fn path_matches(candidate: Option<&Path>, selected: &str, is_directory: bool) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    let selected = Path::new(selected);

    if is_directory {
        candidate.starts_with(selected)
    } else {
        candidate == selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit, commit_file, create_file, create_test_repo};

    #[test]
    fn current_files_include_tracked_and_untracked_but_not_deleted() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "src/main.rs", "fn main() {}", "initial");
        commit_file(&repo, "README.md", "readme", "docs");
        create_file(&repo, "notes/todo.txt", "todo");
        std::fs::remove_file(repo.workdir().unwrap().join("README.md")).unwrap();

        let files = current_project_files(&repo).unwrap();

        assert_eq!(
            files,
            vec!["notes/todo.txt".to_string(), "src/main.rs".to_string()]
        );
    }

    #[test]
    fn file_history_only_contains_relevant_commits() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "src/main.rs", "one", "create main");
        commit_file(&repo, "README.md", "docs", "add docs");
        commit_file(&repo, "src/main.rs", "two", "update main");

        let history = path_history(&repo, "src/main.rs", false, 100).unwrap();
        let messages: Vec<_> = history.iter().map(|item| item.message.as_str()).collect();

        assert_eq!(messages, vec!["update main", "create main"]);
    }

    #[test]
    fn directory_history_contains_descendant_changes() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "src/main.rs", "main", "main");
        commit_file(&repo, "src/lib.rs", "lib", "lib");
        commit_file(&repo, "README.md", "docs", "docs");

        let history = path_history(&repo, "src", true, 100).unwrap();
        let messages: Vec<_> = history.iter().map(|item| item.message.as_str()).collect();

        assert_eq!(messages, vec!["lib", "main"]);
    }

    #[test]
    fn file_history_follows_renames() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "old.txt", "one", "create old");
        std::fs::rename(
            repo.workdir().unwrap().join("old.txt"),
            repo.workdir().unwrap().join("new.txt"),
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("old.txt")).unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        commit(&repo, "rename file");
        commit_file(&repo, "new.txt", "two", "update new");

        let history = path_history(&repo, "new.txt", false, 100).unwrap();
        let messages: Vec<_> = history.iter().map(|item| item.message.as_str()).collect();

        assert_eq!(messages, vec!["update new", "rename file", "create old"]);
    }

    #[test]
    fn compared_history_contains_only_divergent_commits_for_path() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "shared.txt", "base", "common path");
        let common = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &common, false).unwrap();
        drop(common);

        commit_file(&repo, "shared.txt", "main", "main path");
        commit_file(&repo, "README.md", "main docs", "main unrelated");
        crate::git::branch::checkout_branch(&repo, "feature").unwrap();
        commit_file(&repo, "shared.txt", "feature", "feature path");
        commit_file(&repo, "docs.md", "feature docs", "feature unrelated");
        crate::git::branch::checkout_branch(&repo, "main").unwrap();

        let comparison = compare_path_history(&repo, "shared.txt", false, "feature", 100).unwrap();

        assert_eq!((comparison.ahead, comparison.behind), (1, 1));
        let mut commits = comparison
            .commits
            .iter()
            .map(|entry| (entry.commit.message.as_str(), entry.side))
            .collect::<Vec<_>>();
        commits.sort_by_key(|(message, _)| *message);
        assert_eq!(
            commits,
            vec![
                ("feature path", PathHistorySide::Target),
                ("main path", PathHistorySide::Current),
            ]
        );
    }

    #[test]
    fn compared_history_follows_rename_on_current_branch() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "old.txt", "one", "common old");
        let common = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &common, false).unwrap();
        drop(common);

        std::fs::rename(
            repo.workdir().unwrap().join("old.txt"),
            repo.workdir().unwrap().join("new.txt"),
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("old.txt")).unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        commit(&repo, "rename on main");
        commit_file(&repo, "new.txt", "two", "update new on main");
        crate::git::branch::checkout_branch(&repo, "feature").unwrap();
        commit_file(&repo, "old.txt", "feature", "update old on feature");
        crate::git::branch::checkout_branch(&repo, "main").unwrap();

        let comparison = compare_path_history(&repo, "new.txt", false, "feature", 100).unwrap();
        let messages = comparison
            .commits
            .iter()
            .map(|entry| entry.commit.message.as_str())
            .collect::<Vec<_>>();

        assert_eq!(comparison.ahead, 2);
        assert_eq!(comparison.behind, 1);
        assert!(messages.contains(&"update new on main"));
        assert!(messages.contains(&"rename on main"));
        assert!(messages.contains(&"update old on feature"));
        assert_eq!(
            comparison
                .commits
                .iter()
                .find(|entry| entry.commit.message == "update old on feature")
                .map(|entry| entry.side),
            Some(PathHistorySide::Target)
        );
    }

    #[test]
    fn compared_history_follows_rename_on_target_branch() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "old.txt", "one", "common old");
        let common = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &common, false).unwrap();
        drop(common);

        commit_file(&repo, "old.txt", "main", "update old on main");
        crate::git::branch::checkout_branch(&repo, "feature").unwrap();
        std::fs::rename(
            repo.workdir().unwrap().join("old.txt"),
            repo.workdir().unwrap().join("new.txt"),
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("old.txt")).unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        commit(&repo, "rename on feature");
        commit_file(&repo, "new.txt", "feature", "update new on feature");
        crate::git::branch::checkout_branch(&repo, "main").unwrap();

        let comparison = compare_path_history(&repo, "old.txt", false, "feature", 100).unwrap();
        let messages = comparison
            .commits
            .iter()
            .map(|entry| entry.commit.message.as_str())
            .collect::<Vec<_>>();

        assert_eq!((comparison.ahead, comparison.behind), (1, 2));
        assert!(messages.contains(&"update old on main"));
        assert!(messages.contains(&"rename on feature"));
        assert!(messages.contains(&"update new on feature"));
    }

    #[test]
    fn reads_file_content_at_a_commit() {
        let (_temp, repo) = create_test_repo();
        let first = commit_file(&repo, "src/main.rs", "version one", "first");
        commit_file(&repo, "src/main.rs", "version two", "second");

        let content = file_content_at_commit(&repo, first, "src/main.rs").unwrap();

        assert_eq!(content.as_deref(), Some("version one"));
    }

    #[test]
    fn missing_file_at_commit_returns_none() {
        let (_temp, repo) = create_test_repo();
        let first = commit_file(&repo, "README.md", "docs", "first");

        assert_eq!(
            file_content_at_commit(&repo, first, "src/main.rs").unwrap(),
            None
        );
    }
}
