//! Opérations de staging partiel par hunk ou par ligne.

use std::path::Path;

use git2::{ApplyLocation, ApplyOptions, Delta, Diff, DiffOptions, Patch, Repository};

use crate::error::{GitSvError, Result};
use crate::git::diff::DiffLineSelection;

/// Stage un hunk du diff entre l'index et le working tree.
pub fn stage_hunk(repo: &Repository, path: &str, hunk_index: usize) -> Result<()> {
    let mut options = unstaged_diff_options(path);
    let diff = repo.diff_index_to_workdir(None, Some(&mut options))?;
    apply_hunk(repo, &diff, hunk_index)
}

/// Retire de l'index un hunk du diff entre HEAD et l'index.
pub fn unstage_hunk(repo: &Repository, path: &str, hunk_index: usize) -> Result<()> {
    let head_tree = head_tree(repo)?;
    let mut options = DiffOptions::new();
    options
        .pathspec(path)
        .disable_pathspec_match(true)
        .reverse(true);
    let diff = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut options))?;
    apply_hunk(repo, &diff, hunk_index)
}

/// Stage une seule ligne ajoutée ou supprimée.
pub fn stage_line(repo: &Repository, path: &str, selection: DiffLineSelection) -> Result<()> {
    let mut options = unstaged_diff_options(path);
    let diff = repo.diff_index_to_workdir(None, Some(&mut options))?;
    apply_line(repo, &diff, path, selection, false)
}

/// Retire de l'index une seule ligne ajoutée ou supprimée.
pub fn unstage_line(repo: &Repository, path: &str, selection: DiffLineSelection) -> Result<()> {
    let head_tree = head_tree(repo)?;
    let mut options = DiffOptions::new();
    options.pathspec(path).disable_pathspec_match(true);
    let diff = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut options))?;
    apply_line(repo, &diff, path, selection, true)
}

fn unstaged_diff_options(path: &str) -> DiffOptions {
    let mut options = DiffOptions::new();
    options
        .pathspec(path)
        .disable_pathspec_match(true)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    options
}

fn head_tree(repo: &Repository) -> Result<Option<git2::Tree<'_>>> {
    match repo.head() {
        Ok(head) => Ok(Some(head.peel_to_tree()?)),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn apply_hunk(repo: &Repository, diff: &Diff<'_>, target_hunk: usize) -> Result<()> {
    let mut hunk_count = 0;
    for index in 0..diff.deltas().len() {
        if let Some(patch) = Patch::from_diff(diff, index)? {
            hunk_count += patch.num_hunks();
        }
    }
    if target_hunk >= hunk_count {
        return Err(GitSvError::Other("Hunk sélectionné introuvable".into()));
    }
    let mut current_hunk = 0;
    let mut options = ApplyOptions::new();
    options.hunk_callback(|_| {
        let selected = current_hunk == target_hunk;
        current_hunk += 1;
        selected
    });
    repo.apply(diff, ApplyLocation::Index, Some(&mut options))?;
    Ok(())
}

fn apply_line(
    repo: &Repository,
    diff: &Diff<'_>,
    path: &str,
    selection: DiffLineSelection,
    reverse: bool,
) -> Result<()> {
    let patch = find_patch(diff, path)?;
    let delta = patch.delta();
    if !matches!(delta.status(), Delta::Modified) {
        return Err(GitSvError::Other(
            "Le staging par ligne est disponible pour les fichiers modifiés".into(),
        ));
    }

    let patch_buffer = build_line_patch(&patch, path, selection, reverse)?;
    let selected_diff = Diff::from_buffer(&patch_buffer)?;
    repo.apply(&selected_diff, ApplyLocation::Index, None)?;
    Ok(())
}

fn find_patch<'diff>(diff: &'diff Diff<'_>, path: &str) -> Result<Patch<'diff>> {
    for (delta_index, delta) in diff.deltas().enumerate() {
        let matches = delta.new_file().path() == Some(Path::new(path))
            || delta.old_file().path() == Some(Path::new(path));
        if matches {
            return Patch::from_diff(diff, delta_index)?
                .ok_or_else(|| GitSvError::Other("Patch binaire non modifiable".into()));
        }
    }
    Err(GitSvError::Other("Fichier sélectionné introuvable".into()))
}

fn build_line_patch(
    patch: &Patch<'_>,
    path: &str,
    selection: DiffLineSelection,
    reverse: bool,
) -> Result<Vec<u8>> {
    let (hunk, _) = patch.hunk(selection.hunk_index)?;
    let line_count = patch.num_lines_in_hunk(selection.hunk_index)?;
    let mut selected_found = false;
    let mut current_change = 0;
    let mut lines = Vec::new();

    for line_index in 0..line_count {
        let line = patch.line_in_hunk(selection.hunk_index, line_index)?;
        let origin = line.origin();
        let is_change = matches!(origin, '+' | '-');
        let selected = is_change && current_change == selection.change_index;
        selected_found |= selected;

        let output_origin = if reverse {
            match (origin, selected) {
                (' ', _) => Some(' '),
                ('+', true) => Some('-'),
                ('+', false) => Some(' '),
                ('-', true) => Some('+'),
                ('-', false) => None,
                _ => None,
            }
        } else {
            match (origin, selected) {
                (' ', _) => Some(' '),
                ('+', true) => Some('+'),
                ('+', false) => None,
                ('-', true) => Some('-'),
                ('-', false) => Some(' '),
                _ => None,
            }
        };

        if let Some(output_origin) = output_origin {
            lines.push((output_origin, line.content().to_vec()));
        }
        if is_change {
            current_change += 1;
        }
    }

    if !selected_found {
        return Err(GitSvError::Other(
            "Sélectionnez une ligne ajoutée ou supprimée".into(),
        ));
    }

    let old_lines = lines.iter().filter(|(origin, _)| *origin != '+').count();
    let new_lines = lines.iter().filter(|(origin, _)| *origin != '-').count();
    let old_start = if reverse {
        hunk.new_start()
    } else {
        hunk.old_start()
    };
    let new_start = if new_lines == 0 {
        old_start.saturating_sub(1)
    } else if old_lines == 0 {
        old_start + 1
    } else {
        old_start
    };
    let old_path = quote_patch_path(&format!("a/{path}"));
    let new_path = quote_patch_path(&format!("b/{path}"));

    let mut buffer = format!(
        "diff --git {old_path} {new_path}\n--- {old_path}\n+++ {new_path}\n@@ -{old_start},{old_lines} +{new_start},{new_lines} @@\n"
    )
    .into_bytes();
    for (origin, content) in lines {
        buffer.push(origin as u8);
        buffer.extend_from_slice(&content);
        if !content.ends_with(b"\n") {
            buffer.extend_from_slice(b"\n\\ No newline at end of file\n");
        }
    }
    Ok(buffer)
}

fn quote_patch_path(path: &str) -> String {
    let mut quoted = String::from("\"");
    for character in path.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff::{staged_file_diff, working_dir_file_diff};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn partial_staging_preserves_unicode_crlf_and_control_characters_in_paths() {
        for path in ["été 😀 [a].txt", "tab\tquote\"newline\n.txt"] {
            let (directory, repo) = repository_with_file("sibling\n");
            repo.config()
                .unwrap()
                .set_bool("core.autocrlf", false)
                .unwrap();
            let original = "début\r\nfin\r\n";
            let modified = "début\r\n新しい ligne\r\nfin\r\n";
            crate::git::tests::test_utils::commit_file(&repo, path, original, "unicode");
            fs::write(directory.path().join(path), modified).unwrap();
            let before = repo.index().unwrap().write_tree().unwrap();
            let selected = DiffLineSelection {
                hunk_index: 0,
                change_index: 0,
            };
            stage_line(&repo, path, selected).unwrap();
            let entry = repo.index().unwrap().get_path(Path::new(path), 0).unwrap();
            assert_eq!(
                repo.find_blob(entry.id).unwrap().content(),
                modified.as_bytes()
            );
            unstage_line(&repo, path, selected).unwrap();
            assert_eq!(repo.index().unwrap().write_tree().unwrap(), before);
            assert_eq!(
                fs::read(directory.path().join(path)).unwrap(),
                modified.as_bytes()
            );
        }
    }

    #[test]
    fn partial_binary_staging_refuses_without_changing_index_or_worktree() {
        let (directory, repo) = repository_with_file("old\0binary");
        fs::write(directory.path().join("file.txt"), b"new\0binary").unwrap();
        let before = repo.index().unwrap().write_tree().unwrap();
        assert!(stage_line(
            &repo,
            "file.txt",
            DiffLineSelection {
                hunk_index: 0,
                change_index: 0
            }
        )
        .is_err());
        assert!(stage_hunk(&repo, "file.txt", 0).is_err());
        assert_eq!(repo.index().unwrap().write_tree().unwrap(), before);
        assert_eq!(
            fs::read(directory.path().join("file.txt")).unwrap(),
            b"new\0binary"
        );
    }

    #[test]
    fn partial_staging_with_a_locked_index_preserves_disk_and_cache() {
        for by_line in [false, true] {
            let (directory, repo) = repository_with_file("base\n");
            fs::write(directory.path().join("file.txt"), "base\nadded\n").unwrap();
            let before = repo.index().unwrap().write_tree().unwrap();
            let before_bytes = fs::read(repo.path().join("index")).unwrap();
            fs::write(repo.path().join("index.lock"), b"lock").unwrap();
            let result = if by_line {
                stage_line(
                    &repo,
                    "file.txt",
                    DiffLineSelection {
                        hunk_index: 0,
                        change_index: 0,
                    },
                )
            } else {
                stage_hunk(&repo, "file.txt", 0)
            };
            assert!(result.is_err());
            assert_eq!(fs::read(repo.path().join("index")).unwrap(), before_bytes);
            assert_eq!(repo.index().unwrap().write_tree().unwrap(), before);
            assert_eq!(
                fs::read(directory.path().join("file.txt")).unwrap(),
                b"base\nadded\n"
            );
        }
    }

    #[test]
    fn missing_head_does_not_discard_a_staged_file_as_an_initial_commit() {
        let (directory, repo) = repository_with_file("base\n");
        fs::write(directory.path().join("file.txt"), "staged\n").unwrap();
        crate::git::commit::stage_file(&repo, "file.txt").unwrap();
        let before = repo.index().unwrap().write_tree().unwrap();
        fs::remove_file(repo.path().join("HEAD")).unwrap();
        assert!(unstage_hunk(&repo, "file.txt", 0).is_err());
        assert_eq!(repo.index().unwrap().write_tree().unwrap(), before);
        assert_eq!(
            fs::read(directory.path().join("file.txt")).unwrap(),
            b"staged\n"
        );
    }

    #[test]
    fn invalid_hunk_does_not_remove_a_staged_file() {
        let (directory, repo) = repository_with_file("base\n");
        fs::write(directory.path().join("new.txt"), "new\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        let before = index.write_tree().unwrap();
        assert!(unstage_hunk(&repo, "new.txt", 1).is_err());
        assert_eq!(repo.index().unwrap().write_tree().unwrap(), before);
    }

    #[test]
    fn stage_line_after_an_unselected_hunk_uses_index_line_numbers() {
        let original = (0..30)
            .map(|line| format!("line {line}\n"))
            .collect::<String>();
        let (directory, repo) = repository_with_file(&original);
        let modified = original
            .replace("line 1\n", "first addition\nline 1\n")
            .replace("line 25\n", "later addition\nline 25\n");
        fs::write(directory.path().join("file.txt"), modified).unwrap();
        stage_line(
            &repo,
            "file.txt",
            DiffLineSelection {
                hunk_index: 1,
                change_index: 0,
            },
        )
        .unwrap();
        assert_eq!(
            index_content(&repo),
            original.replace("line 25\n", "later addition\nline 25\n")
        );
    }

    #[test]
    fn stage_line_quotes_paths_containing_spaces() {
        let (directory, repo) = repository_with_file("base\n");
        crate::git::tests::test_utils::commit_file(
            &repo,
            "with space.txt",
            "alpha\nomega\n",
            "spaces",
        );
        fs::write(
            directory.path().join("with space.txt"),
            "alpha\nbeta\nomega\n",
        )
        .unwrap();
        stage_line(
            &repo,
            "with space.txt",
            DiffLineSelection {
                hunk_index: 0,
                change_index: 0,
            },
        )
        .unwrap();
        let entry = repo
            .index()
            .unwrap()
            .get_path(Path::new("with space.txt"), 0)
            .unwrap();
        assert_eq!(
            repo.find_blob(entry.id).unwrap().content(),
            b"alpha\nbeta\nomega\n"
        );
    }

    #[test]
    fn stage_line_preserves_missing_final_newline() {
        let (directory, repo) = repository_with_file("alpha\nomega");
        fs::write(directory.path().join("file.txt"), "alpha\nbeta\nomega").unwrap();
        stage_line(
            &repo,
            "file.txt",
            DiffLineSelection {
                hunk_index: 0,
                change_index: 0,
            },
        )
        .unwrap();
        assert_eq!(index_content(&repo), "alpha\nbeta\nomega");
        unstage_line(
            &repo,
            "file.txt",
            DiffLineSelection {
                hunk_index: 0,
                change_index: 0,
            },
        )
        .unwrap();
        assert_eq!(index_content(&repo), "alpha\nomega");
    }

    #[test]
    fn unstage_hunk_works_before_first_commit() {
        let directory = TempDir::new().unwrap();
        let repo = Repository::init(directory.path()).unwrap();
        fs::write(directory.path().join("file.txt"), "new\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        unstage_hunk(&repo, "file.txt", 0).unwrap();
        assert!(repo.index().unwrap().is_empty());
        assert_eq!(
            fs::read_to_string(directory.path().join("file.txt")).unwrap(),
            "new\n"
        );
    }

    #[test]
    fn stage_hunk_uses_literal_path() {
        let (directory, repo) = repository_with_file("base\n");
        crate::git::tests::test_utils::commit_file(&repo, "[a].txt", "base\n", "literal");
        crate::git::tests::test_utils::commit_file(&repo, "a.txt", "other\n", "other");
        fs::write(directory.path().join("[a].txt"), "selected\n").unwrap();
        fs::write(directory.path().join("a.txt"), "unselected\n").unwrap();
        stage_hunk(&repo, "[a].txt", 0).unwrap();
        let index = repo.index().unwrap();
        let selected = index.get_path(Path::new("[a].txt"), 0).unwrap();
        let other = index.get_path(Path::new("a.txt"), 0).unwrap();
        assert_eq!(
            repo.find_blob(selected.id).unwrap().content(),
            b"selected\n"
        );
        assert_eq!(repo.find_blob(other.id).unwrap().content(), b"other\n");
    }

    fn repository_with_file(content: &str) -> (TempDir, Repository) {
        let directory = TempDir::new().unwrap();
        let repo = Repository::init(directory.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        fs::write(directory.path().join("file.txt"), content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "Initial", &tree, &[])
            .unwrap();
        drop(tree);
        (directory, repo)
    }

    fn index_content(repo: &Repository) -> String {
        let index = repo.index().unwrap();
        let entry = index.get_path(Path::new("file.txt"), 0).unwrap();
        String::from_utf8(repo.find_blob(entry.id).unwrap().content().to_vec()).unwrap()
    }

    #[test]
    fn test_stage_and_unstage_selected_hunk() {
        let (directory, repo) = repository_with_file(
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\nthirteen\nfourteen\nfifteen\n",
        );
        fs::write(
            directory.path().join("file.txt"),
            "one\nTWO\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\nTWELVE\nthirteen\nfourteen\nfifteen\n",
        )
        .unwrap();

        stage_hunk(&repo, "file.txt", 0).unwrap();
        let staged = index_content(&repo);
        assert!(staged.contains("TWO"));
        assert!(staged.contains("twelve"));

        unstage_hunk(&repo, "file.txt", 0).unwrap();
        assert_eq!(
            index_content(&repo),
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve\nthirteen\nfourteen\nfifteen\n"
        );
    }

    #[test]
    fn test_stage_and_unstage_single_added_line() {
        let (directory, repo) = repository_with_file("alpha\nomega\n");
        fs::write(
            directory.path().join("file.txt"),
            "alpha\nbeta\ngamma\nomega\n",
        )
        .unwrap();

        let unstaged = working_dir_file_diff(&repo, "file.txt").unwrap();
        let beta_line = unstaged
            .lines
            .iter()
            .position(|line| line.content == "beta")
            .unwrap();
        stage_line(
            &repo,
            "file.txt",
            unstaged.change_at_line(beta_line).unwrap(),
        )
        .unwrap();
        assert_eq!(index_content(&repo), "alpha\nbeta\nomega\n");

        let staged = staged_file_diff(&repo, "file.txt").unwrap();
        let beta_line = staged
            .lines
            .iter()
            .position(|line| line.content == "beta")
            .unwrap();
        unstage_line(&repo, "file.txt", staged.change_at_line(beta_line).unwrap()).unwrap();
        assert_eq!(index_content(&repo), "alpha\nomega\n");
    }
}
