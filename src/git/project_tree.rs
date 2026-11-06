//! Accès à l'arborescence courante et à l'historique par chemin.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::Path;

use git2::{Delta, ErrorCode, IndexEntryExtendedFlag, IndexEntryFlag, Oid, Repository, Sort, Tree};

use crate::error::Result;

use super::commit::CommitInfo;
use super::repo::StatusEntry;

/// Taille maximale d'un contenu destiné au presse-papier.
const MAX_FILE_CONTENT_BYTES: usize = 20 * 1_048_576;

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
    if repo.workdir().is_none() {
        return Ok(Vec::new());
    }
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let entries: Vec<_> = repo
        .statuses(Some(&mut options))?
        .iter()
        .map(|entry| {
            Ok(StatusEntry {
                path: entry
                    .path()
                    .map_err(|_| crate::error::GitSvError::UnsupportedPathEncoding)?
                    .to_string(),
                status: entry.status(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    current_project_files_from_status(repo, &entries)
}

/// Construit l'arborescence depuis un status complet déjà chargé pour ce worktree.
///
/// Le status doit inclure les fichiers non suivis récursivement, sans détection
/// des renommages. Ses suppressions évitent de vérifier chaque fichier de l'index.
pub fn current_project_files_from_status(
    repo: &Repository,
    entries: &[StatusEntry],
) -> Result<Vec<String>> {
    let Some(workdir) = repo.workdir() else {
        return Ok(Vec::new());
    };
    let mut files = BTreeSet::new();
    for entry in repo.index()?.iter() {
        let path = std::str::from_utf8(&entry.path)
            .map_err(|_| crate::error::GitSvError::UnsupportedPathEncoding)?;
        // Ces entrées échappent au scan du status, même si le fichier a disparu.
        // Un sous-module absent peut aussi être considéré comme non initialisé.
        let needs_metadata = entry.flags & IndexEntryFlag::VALID.bits() != 0
            || entry.flags_extended & IndexEntryExtendedFlag::SKIP_WORKTREE.bits() != 0
            || entry.mode == 0o160000;
        if !needs_metadata || std::fs::symlink_metadata(workdir.join(path)).is_ok() {
            files.insert(path.to_string());
        }
    }
    for entry in entries {
        if entry.status.is_wt_deleted()
            || (entry.status.is_conflicted()
                && std::fs::symlink_metadata(workdir.join(&entry.path)).is_err())
        {
            files.remove(&entry.path);
        } else if entry.status.is_wt_new() {
            files.insert(entry.path.clone());
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

    let head = match repo.head() {
        Ok(head) => head.peel_to_commit()?,
        Err(error) if error.code() == ErrorCode::UnbornBranch => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    path_history_at(repo, head.id(), path, is_directory, max_count)
}

/// Lit un historique depuis un HEAD figé pendant un chargement asynchrone.
pub(crate) fn path_history_at(
    repo: &Repository,
    head: Oid,
    path: &str,
    is_directory: bool,
    max_count: usize,
) -> Result<Vec<CommitInfo>> {
    if path.is_empty() || max_count == 0 {
        return Ok(Vec::new());
    }
    path_history_from(repo, head, None, path, is_directory, max_count)
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
    compare_path_history_at(repo, head.id(), target.id(), path, is_directory, max_count)
}

/// Compare deux pointes figées pour ne pas mélanger les références pendant le calcul.
pub(crate) fn compare_path_history_at(
    repo: &Repository,
    head: Oid,
    target: Oid,
    path: &str,
    is_directory: bool,
    max_count: usize,
) -> Result<PathHistoryComparison> {
    let target_path = match repo.merge_base(head, target) {
        Ok(merge_base) => {
            let common_path = path_at_ancestor(repo, head, merge_base, path, is_directory)?;
            path_at_descendant(repo, target, merge_base, &common_path, is_directory)?
        }
        Err(_) => path.to_string(),
    };
    let current_history =
        path_history_from(repo, head, Some(target), path, is_directory, max_count)?;
    let target_history = path_history_from(
        repo,
        target,
        Some(head),
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
        // Les mêmes identités servent à détecter les modifications et les renommages.
        let paths = paths
            .into_iter()
            .map(|path| {
                let current_entry = tree_entry_identity(Some(&tree), &path)?;
                let parent_entry = tree_entry_identity(parent_tree.as_ref(), &path)?;
                Ok((path, current_entry, parent_entry))
            })
            .collect::<Result<Vec<_>>>()?;
        let matched = paths
            .iter()
            .any(|(_, current_entry, parent_entry)| current_entry != parent_entry);

        if matched {
            history.push(CommitInfo::from_git2_commit(&commit));
            if history.len() >= max_count {
                break;
            }
        }

        for (parent_index, parent) in commit.parents().enumerate() {
            let parent_tree = parent.tree()?;
            let parent_paths = tracked_paths.entry(parent.id()).or_default();
            for (tracked_path, current_entry, first_parent_entry) in &paths {
                let follow_rename = if is_directory || current_entry.is_none() {
                    false
                } else if parent_index == 0 {
                    first_parent_entry.is_none()
                } else {
                    tree_entry_identity(Some(&parent_tree), tracked_path)?.is_none()
                };
                parent_paths.insert(resolve_previous_path(
                    repo,
                    &parent_tree,
                    &tree,
                    tracked_path,
                    follow_rename,
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
    let mut pending = vec![(descendant, None)];
    let mut towards_descendant = HashMap::new();

    // Un parcours unique évite une nouvelle recherche d'ascendance à chaque commit.
    while let Some((current, child)) = pending.pop() {
        if towards_descendant.contains_key(&current) {
            continue;
        }
        towards_descendant.insert(current, child);
        if current == ancestor {
            let mut chain = vec![current];
            let mut cursor = current;
            while let Some(child) = towards_descendant[&cursor] {
                chain.push(child);
                cursor = child;
            }
            chain.reverse();
            return Ok(chain);
        }

        let commit = repo.find_commit(current)?;
        // L'ordre inverse dans la pile conserve la préférence du premier parent.
        let parents: Vec<_> = commit.parent_ids().collect();
        pending.extend(
            parents
                .into_iter()
                .rev()
                .map(|parent| (parent, Some(current))),
        );
    }

    Err(git2::Error::from_str("ancetre inaccessible depuis le commit").into())
}

fn previous_path(
    repo: &Repository,
    parent_tree: &Tree<'_>,
    tree: &Tree<'_>,
    path: &str,
    is_directory: bool,
) -> Result<String> {
    let follow_rename = !is_directory
        && tree_entry_identity(Some(tree), path)?.is_some()
        && tree_entry_identity(Some(parent_tree), path)?.is_none();
    resolve_previous_path(repo, parent_tree, tree, path, follow_rename)
}

fn resolve_previous_path(
    repo: &Repository,
    parent_tree: &Tree<'_>,
    tree: &Tree<'_>,
    path: &str,
    follow_rename: bool,
) -> Result<String> {
    if follow_rename {
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

    for delta in diff.deltas() {
        if delta.status() == Delta::Renamed && path_matches(delta.new_file().path(), path, false) {
            return delta
                .old_file()
                .path()
                .map(|value| super::helpers::path_as_utf8(value).map(str::to_owned))
                .transpose();
        }
    }
    Ok(None)
}

fn renamed_to_path(
    repo: &Repository,
    parent_tree: &Tree<'_>,
    tree: &Tree<'_>,
    path: &str,
) -> Result<Option<String>> {
    let mut diff = repo.diff_tree_to_tree(Some(parent_tree), Some(tree), None)?;
    diff.find_similar(None)?;

    for delta in diff.deltas() {
        if delta.status() == Delta::Renamed && path_matches(delta.old_file().path(), path, false) {
            return delta
                .new_file()
                .path()
                .map(|value| super::helpers::path_as_utf8(value).map(str::to_owned))
                .transpose();
        }
    }
    Ok(None)
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
    if repo.odb()?.read_header(entry.id())?.0 > MAX_FILE_CONTENT_BYTES {
        return Err(crate::error::GitSvError::Other(format!(
            "Le fichier '{path}' est trop volumineux pour le presse-papier (limite : 20 MiB)"
        )));
    }
    let object = entry.to_object(repo)?;
    let blob = object.as_blob().ok_or_else(|| {
        crate::error::GitSvError::Other(format!("Le chemin '{path}' n'est pas un fichier"))
    })?;
    if blob.is_binary() {
        return Err(crate::error::GitSvError::Other(format!(
            "Le fichier '{path}' est binaire"
        )));
    }
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

    #[cfg(unix)]
    #[test]
    fn safety_project_files_reject_lossy_path_collisions() {
        let (_temp, repo) = crate::git::tests::test_utils::create_test_repo();
        crate::git::tests::test_utils::commit_file(&repo, "bad�.txt", "valid name", "initial");
        crate::git::tests::test_utils::add_raw_index_file(&repo, b"bad\xff.txt", b"raw name");
        assert!(matches!(
            current_project_files_from_status(&repo, &[]),
            Err(crate::error::GitSvError::UnsupportedPathEncoding)
        ));
    }
    use crate::git::tests::test_utils::{commit, commit_file, create_file, create_test_repo};

    #[test]
    #[ignore = "mesure manuelle en lecture seule via GIT_SV_PROFILE_REPO"]
    fn profile_large_repository_views() {
        let path = std::env::var("GIT_SV_PROFILE_REPO").expect("GIT_SV_PROFILE_REPO requis");
        let mut repo = Repository::open(path).unwrap();
        for pass in 1..=2 {
            let started = std::time::Instant::now();
            let (local, remote) = crate::git::branch::list_all_branches(&repo).unwrap();
            eprintln!(
                "pass={pass} branches_ms={} local={} remote={}",
                started.elapsed().as_millis(),
                local.len(),
                remote.len()
            );
            let started = std::time::Instant::now();
            let files = current_project_files(&repo).unwrap();
            eprintln!(
                "pass={pass} project_files_ms={} files={}",
                started.elapsed().as_millis(),
                files.len()
            );
            let mut options = git2::StatusOptions::new();
            options.include_untracked(true).recurse_untracked_dirs(true);
            let status: Vec<_> = repo
                .statuses(Some(&mut options))
                .unwrap()
                .iter()
                .filter_map(|entry| {
                    entry.path().ok().map(|path| StatusEntry {
                        path: path.to_string(),
                        status: entry.status(),
                    })
                })
                .collect();
            let started = std::time::Instant::now();
            let cached_files = current_project_files_from_status(&repo, &status).unwrap();
            eprintln!(
                "pass={pass} project_files_cached_status_ms={} files={}",
                started.elapsed().as_millis(),
                cached_files.len()
            );
            assert_eq!(files, cached_files);
            let started = std::time::Instant::now();
            let stashes = crate::git::stash::list_stashes(&mut repo).unwrap();
            eprintln!(
                "pass={pass} stashes_ms={} stashes={}",
                started.elapsed().as_millis(),
                stashes.len()
            );
            let started = std::time::Instant::now();
            let worktrees = crate::git::worktree::list_worktrees(&repo).unwrap();
            eprintln!(
                "pass={pass} worktrees_ms={} worktrees={}",
                started.elapsed().as_millis(),
                worktrees.len()
            );
        }
    }

    #[test]
    #[ignore = "mesure manuelle en lecture seule via GIT_SV_PROFILE_REPO"]
    fn profile_large_repository_path_history() {
        let path = std::env::var("GIT_SV_PROFILE_REPO").expect("GIT_SV_PROFILE_REPO requis");
        let repo = Repository::open(path).unwrap();
        let index = repo.index().unwrap();
        let files: Vec<_> = index
            .iter()
            .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
            .collect();
        let file = &files[files.len() / 2];
        let directory = files
            .iter()
            .find_map(|path| path.split_once('/').map(|(directory, _)| directory))
            .unwrap();
        for pass in 1..=2 {
            for (kind, path, is_directory) in [
                ("file", file.as_str(), false),
                ("directory", directory, true),
            ] {
                let started = std::time::Instant::now();
                let history = path_history(&repo, path, is_directory, 100).unwrap();
                eprintln!(
                    "pass={pass} history_kind={kind} history_ms={} commits={}",
                    started.elapsed().as_millis(),
                    history.len()
                );
            }
        }
    }

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
    fn current_files_handle_staged_renames_deletions_and_ignored_files() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "old.txt", "renamed", "initial");
        commit_file(&repo, "recreated.txt", "original", "tracked");
        commit_file(&repo, ".gitignore", "ignored/\n", "ignore");
        let workdir = repo.workdir().unwrap();
        std::fs::rename(workdir.join("old.txt"), workdir.join("new.txt")).unwrap();
        create_file(&repo, "deleted-after-staging.txt", "new");
        create_file(&repo, "ignored/output.txt", "ignored");

        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("old.txt")).unwrap();
        index.remove_path(Path::new("recreated.txt")).unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index
            .add_path(Path::new("deleted-after-staging.txt"))
            .unwrap();
        index.write().unwrap();
        std::fs::remove_file(workdir.join("deleted-after-staging.txt")).unwrap();

        assert_eq!(
            current_project_files(&repo).unwrap(),
            vec![".gitignore", "new.txt", "recreated.txt"]
        );
    }

    #[test]
    fn current_files_check_paths_hidden_from_status() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "assumed.txt", "valid", "initial");
        commit_file(&repo, "sparse.txt", "sparse", "sparse");
        commit_file(&repo, "submodule", "placeholder", "submodule");
        commit_file(&repo, "present.txt", "present", "present");
        let mut index = repo.index().unwrap();
        let mut assumed = index.get_path(Path::new("assumed.txt"), 0).unwrap();
        assumed.flags |= IndexEntryFlag::VALID.bits();
        index.add(&assumed).unwrap();
        let mut sparse = index.get_path(Path::new("sparse.txt"), 0).unwrap();
        sparse.flags_extended |= IndexEntryExtendedFlag::SKIP_WORKTREE.bits();
        index.add(&sparse).unwrap();
        let mut submodule = index.get_path(Path::new("submodule"), 0).unwrap();
        submodule.mode = 0o160000;
        submodule.id = repo.head().unwrap().target().unwrap();
        index.add(&submodule).unwrap();
        index.write().unwrap();
        std::fs::remove_file(repo.workdir().unwrap().join("assumed.txt")).unwrap();
        std::fs::remove_file(repo.workdir().unwrap().join("sparse.txt")).unwrap();
        std::fs::remove_file(repo.workdir().unwrap().join("submodule")).unwrap();

        assert_eq!(
            current_project_files_from_status(&repo, &[]).unwrap(),
            vec!["present.txt"]
        );
        assert_eq!(current_project_files(&repo).unwrap(), vec!["present.txt"]);
    }

    #[test]
    fn current_files_exclude_missing_conflicted_paths() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "conflict.txt", "conflict", "initial");
        std::fs::remove_file(repo.workdir().unwrap().join("conflict.txt")).unwrap();
        let status = vec![StatusEntry {
            path: "conflict.txt".into(),
            status: git2::Status::CONFLICTED,
        }];

        assert!(current_project_files_from_status(&repo, &status)
            .unwrap()
            .is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn current_files_include_dangling_symlinks() {
        let (_temp, repo) = create_test_repo();
        std::os::unix::fs::symlink("missing", repo.workdir().unwrap().join("link")).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("link")).unwrap();
        index.write().unwrap();

        assert_eq!(current_project_files(&repo).unwrap(), vec!["link"]);
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

    #[test]
    fn audit_unborn_repository_has_empty_path_history() {
        let (_temp, repo) = create_test_repo();
        create_file(&repo, "new.txt", "new");

        assert!(path_history(&repo, "new.txt", false, 100)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn audit_ancestry_chain_handles_second_parent_and_unrelated_roots() {
        let (_temp, repo) = create_test_repo();
        let ancestor_id = commit_file(&repo, "file.txt", "content", "ancestor");
        let ancestor = repo.find_commit(ancestor_id).unwrap();
        let tree = ancestor.tree().unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let other_id = repo
            .commit(None, &sig, &sig, "other root", &tree, &[])
            .unwrap();
        let other = repo.find_commit(other_id).unwrap();
        let merged = repo
            .commit(None, &sig, &sig, "merge", &tree, &[&other, &ancestor])
            .unwrap();

        assert_eq!(
            ancestry_chain(&repo, merged, ancestor_id).unwrap(),
            vec![merged, ancestor_id]
        );
        assert!(ancestry_chain(&repo, other_id, ancestor_id).is_err());
    }

    #[test]
    fn audit_ancestry_chain_prefers_first_parent_even_if_second_is_shorter() {
        let (_temp, repo) = create_test_repo();
        let ancestor_id = commit_file(&repo, "file.txt", "content", "ancestor");
        let ancestor = repo.find_commit(ancestor_id).unwrap();
        let tree = ancestor.tree().unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let first_id = repo
            .commit(None, &sig, &sig, "first", &tree, &[&ancestor])
            .unwrap();
        let first = repo.find_commit(first_id).unwrap();
        let merged = repo
            .commit(None, &sig, &sig, "merge", &tree, &[&first, &ancestor])
            .unwrap();

        assert_eq!(
            ancestry_chain(&repo, merged, ancestor_id).unwrap(),
            vec![merged, first_id, ancestor_id]
        );
    }

    #[test]
    fn audit_clipboard_rejects_nul_containing_binary() {
        let (_temp, repo) = create_test_repo();
        let oid = commit_file(&repo, "binary.dat", "valid utf8\0binary", "binary");

        assert!(file_content_at_commit(&repo, oid, "binary.dat")
            .unwrap_err()
            .to_string()
            .contains("binaire"));
    }
}
