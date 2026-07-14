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
    let head_tree = repo.head()?.peel_to_tree()?;
    let mut options = DiffOptions::new();
    options.pathspec(path).reverse(true);
    let diff = repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut options))?;
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
    let head_tree = repo.head()?.peel_to_tree()?;
    let mut options = DiffOptions::new();
    options.pathspec(path);
    let diff = repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut options))?;
    apply_line(repo, &diff, path, selection, true)
}

fn unstaged_diff_options(path: &str) -> DiffOptions {
    let mut options = DiffOptions::new();
    options
        .pathspec(path)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    options
}

fn apply_hunk(repo: &Repository, diff: &Diff<'_>, target_hunk: usize) -> Result<()> {
    let mut current_hunk = 0;
    let mut found = false;
    let mut options = ApplyOptions::new();
    options.hunk_callback(|_| {
        let selected = current_hunk == target_hunk;
        found |= selected;
        current_hunk += 1;
        selected
    });
    repo.apply(diff, ApplyLocation::Index, Some(&mut options))?;
    drop(options);

    if !found {
        return Err(GitSvError::Other("Hunk sélectionné introuvable".into()));
    }
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
    if path.contains(['\n', '\r']) {
        return Err(GitSvError::Other(
            "Chemin de fichier non pris en charge".into(),
        ));
    }

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
    let (old_start, new_start) = if reverse {
        (hunk.new_start(), hunk.old_start())
    } else {
        (hunk.old_start(), hunk.new_start())
    };

    let mut buffer = format!(
        "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -{old_start},{old_lines} +{new_start},{new_lines} @@\n"
    )
    .into_bytes();
    for (origin, content) in lines {
        buffer.push(origin as u8);
        buffer.extend_from_slice(&content);
        if !content.ends_with(b"\n") {
            buffer.push(b'\n');
        }
    }
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff::{staged_file_diff, working_dir_file_diff};
    use std::fs;
    use tempfile::TempDir;

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
