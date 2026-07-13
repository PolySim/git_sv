//! Accès à l'arborescence courante et à l'historique par chemin.

use std::collections::BTreeSet;
use std::path::Path;

use git2::{Delta, Repository, Sort};

use crate::error::Result;

use super::commit::CommitInfo;

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

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;

    let mut tracked_path = path.to_string();
    let mut history = Vec::new();

    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let mut diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        diff.find_similar(None)?;

        let mut matched = false;
        let mut previous_path = None;

        for delta in diff.deltas() {
            let old_path = delta.old_file().path();
            let new_path = delta.new_file().path();

            if path_matches(old_path, &tracked_path, is_directory)
                || path_matches(new_path, &tracked_path, is_directory)
            {
                matched = true;
            }

            if !is_directory
                && delta.status() == Delta::Renamed
                && path_matches(new_path, &tracked_path, false)
            {
                previous_path = old_path.map(|value| value.to_string_lossy().into_owned());
            }
        }

        if matched {
            history.push(CommitInfo::from_git2_commit(&commit));
            if history.len() >= max_count {
                break;
            }
        }

        if let Some(previous_path) = previous_path {
            tracked_path = previous_path;
        }
    }

    Ok(history)
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
