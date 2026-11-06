//! Opérations stash : listage, sauvegarde, application, suppression.

#![allow(dead_code)]
use std::process::Command;

use git2::{Oid, Repository};

use crate::error::Result;
use crate::git::diff::DiffStatus;

/// Fichier modifié dans un stash.
#[derive(Debug, Clone)]
pub struct StashFile {
    pub path: String,
    pub status: DiffStatus,
}

impl StashFile {
    /// Retourne le caractère représentant le statut du fichier.
    pub fn status_char(&self) -> char {
        match self.status {
            DiffStatus::Added => 'A',
            DiffStatus::Modified => 'M',
            DiffStatus::Deleted => 'D',
            DiffStatus::Renamed => 'R',
        }
    }
}

/// Entrée de stash.
#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    /// Branche sur laquelle le stash a été créé.
    pub branch: Option<String>,
    /// Date de création du stash.
    pub timestamp: Option<i64>,
    /// Fichiers modifiés dans ce stash.
    pub files: Vec<StashFile>,
    /// Oid du commit du stash (pour récupérer les diffs).
    pub oid: Oid,
}

/// Résultat d'une création de stash via la CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashPushOutcome {
    Created,
    NoChanges,
}

impl Default for StashEntry {
    fn default() -> Self {
        Self {
            index: 0,
            message: String::new(),
            branch: None,
            timestamp: None,
            files: Vec::new(),
            oid: Oid::ZERO_SHA1,
        }
    }
}

/// Liste tous les stashes.
pub fn list_stashes(repo: &mut Repository) -> Result<Vec<StashEntry>> {
    // D'abord, collecter les infos de base des stashes (sans les fichiers)
    let mut temp_entries: Vec<(usize, String, Option<String>, Oid)> = Vec::new();

    repo.stash_foreach(|index, message, oid| {
        let branch = extract_branch_from_message(message);
        temp_entries.push((index, message.to_string(), branch, *oid));
        true
    })?;

    // Maintenant charger les fichiers pour chaque stash
    let mut entries = Vec::new();
    for (index, message, branch, oid) in temp_entries {
        let files = stash_files(repo, oid)?;
        entries.push(StashEntry {
            index,
            message,
            branch,
            timestamp: None,
            files,
            oid,
        });
    }

    Ok(entries)
}

/// Extrait le nom de la branche depuis le message de stash.
fn extract_branch_from_message(message: &str) -> Option<String> {
    // Format typique: "WIP on <branch>: ..." ou "On <branch>: ..."
    if let Some(start) = message.find(" on ") {
        let rest = &message[start + 4..];
        if let Some(end) = rest.find(':') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Récupère la liste des fichiers modifiés dans un stash.
pub fn stash_files(repo: &Repository, stash_oid: Oid) -> Result<Vec<StashFile>> {
    let stash_commit = repo.find_commit(stash_oid)?;
    let stash_tree = stash_commit.tree()?;

    // Le parent du stash est le commit sur lequel il a été créé
    let parent = stash_commit.parent(0)?;
    let parent_tree = parent.tree()?;

    let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&stash_tree), None)?;
    let mut files = collect_stash_files(&diff)?;
    if stash_commit.parent_count() > 2 {
        let untracked_tree = stash_commit.parent(2)?.tree()?;
        let untracked_diff = repo.diff_tree_to_tree(None, Some(&untracked_tree), None)?;
        files.extend(collect_stash_files(&untracked_diff)?);
    }
    Ok(files)
}

fn collect_stash_files(diff: &git2::Diff<'_>) -> Result<Vec<StashFile>> {
    let mut files = Vec::new();
    // Les métadonnées suffisent : foreach construirait aussi chaque patch.
    for delta in diff.deltas() {
        let Some(path) = delta.new_file().path() else {
            continue;
        };
        let status = match delta.status() {
            git2::Delta::Added => DiffStatus::Added,
            git2::Delta::Modified => DiffStatus::Modified,
            git2::Delta::Deleted => DiffStatus::Deleted,
            git2::Delta::Renamed => DiffStatus::Renamed,
            _ => continue,
        };
        files.push(StashFile {
            path: crate::git::helpers::path_as_utf8(path)?.to_string(),
            status,
        });
    }
    Ok(files)
}

/// Récupère le diff complet d'un fichier dans un stash.
pub fn stash_file_diff(repo: &Repository, stash_oid: Oid, file_path: &str) -> Result<Vec<String>> {
    let stash_commit = repo.find_commit(stash_oid)?;
    let stash_tree = stash_commit.tree()?;

    let parent = stash_commit.parent(0)?;
    let parent_tree = parent.tree()?;

    let mut options = git2::DiffOptions::new();
    options
        .pathspec(file_path)
        .disable_pathspec_match(true)
        .max_size(4 * 1024 * 1024);
    let untracked_tree = if stash_commit.parent_count() > 2 {
        Some(stash_commit.parent(2)?.tree()?)
    } else {
        None
    };
    let diff = if let Some(tree) = untracked_tree
        .as_ref()
        .filter(|tree| tree.get_path(std::path::Path::new(file_path)).is_ok())
    {
        repo.diff_tree_to_tree(None, Some(tree), Some(&mut options))?
    } else {
        repo.diff_tree_to_tree(Some(&parent_tree), Some(&stash_tree), Some(&mut options))?
    };

    let mut file_lines = Vec::new();
    let mut bytes = 0usize;
    let mut truncated = false;
    let result = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let expanded_size = line.content().len().saturating_mul(4);
        if file_lines.len() >= 20_000 || bytes.saturating_add(expanded_size) > 8 * 1024 * 1024 {
            truncated = true;
            return false;
        }
        let prefix = match line.origin() {
            '+' => "+",
            '-' => "-",
            ' ' => " ",
            _ => "",
        };
        let content = format!(
            "{}{}",
            prefix,
            String::from_utf8_lossy(line.content())
                .trim_end_matches('\n')
                .replace('\t', "    ")
        );
        bytes += content.len();
        file_lines.push(content);
        true
    });
    if truncated {
        file_lines.push("… Diff tronqué : limite d'affichage atteinte".into());
    } else {
        result?;
    }

    Ok(file_lines)
}

/// Sauvegarde le working directory dans un stash.
pub fn save_stash(repo: &mut Repository, message: Option<&str>) -> Result<()> {
    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("git_sv", "git_sv@local"))?;

    let msg = message.unwrap_or("Stash créé par git_sv");
    repo.stash_save(&sig, msg, None)?;
    Ok(())
}

/// Applique un stash sans le supprimer.
pub fn apply_stash(repo: &mut Repository, index: usize) -> Result<()> {
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_apply(index, Some(&mut opts))?;
    // libgit2 peut réussir le checkout tout en laissant des entrées en conflit.
    let mut updated_index = repo.index()?;
    updated_index.read(true)?;
    if updated_index.has_conflicts() {
        return Err(crate::error::GitSvError::Other(
            "Le stash a produit des conflits ; il est conservé jusqu'à leur résolution".into(),
        ));
    }
    Ok(())
}

/// Applique et supprime le stash à l'index donné.
pub fn pop_stash(repo: &mut Repository, index: usize) -> Result<()> {
    let reference = format!("stash@{{{index}}}");
    let selected = repo.revparse_single(&reference)?.id();
    apply_stash(repo, index)?;
    // Une modification concurrente de la pile ne doit pas supprimer un autre stash.
    if repo.revparse_single(&reference)?.id() != selected {
        return Err(crate::error::GitSvError::Other(
            "Stash appliqué mais conservé : la liste des stashes a changé".into(),
        ));
    }
    drop_stash(repo, index)
}

/// Supprime le stash à l'index donné sans l'appliquer.
pub fn drop_stash(repo: &mut Repository, index: usize) -> Result<()> {
    repo.stash_drop(index)?;
    Ok(())
}

/// Exécute `git stash push` via la CLI et interprète le résultat.
fn run_stash_push_command(
    repo_path: &str,
    args: &[&str],
    literal_paths: bool,
) -> Result<StashPushOutcome> {
    let repo = Repository::discover(repo_path)?;
    let previous = repo.refname_to_id("refs/stash").ok();
    let mut cmd = Command::new("git");
    // Sans chemin, stash utilise lui-même :/ pour nettoyer les fichiers sauvés.
    if literal_paths {
        cmd.arg("--literal-pathspecs");
    }
    cmd.args(["stash", "push"]).args(args);
    cmd.current_dir(
        repo.workdir()
            .ok_or_else(|| git2::Error::from_str("Working tree introuvable"))?,
    );

    let output = crate::git::helpers::run_command_with_timeout(
        cmd,
        "stash_push",
        std::time::Duration::from_secs(30),
    )?;
    if !output.status.success() {
        return Err(crate::error::GitSvError::OperationFailed {
            operation: "stash_push",
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(if repo.refname_to_id("refs/stash").ok() == previous {
        StashPushOutcome::NoChanges
    } else {
        StashPushOutcome::Created
    })
}

/// Stash les changements non stagés d'un fichier spécifique en conservant l'index.
///
/// Utilise la CLI Git car libgit2 ne supporte pas ce workflow fin.
pub fn stash_file(
    repo_path: &str,
    file_path: &str,
    message: Option<&str>,
) -> Result<StashPushOutcome> {
    ensure_stash_path_has_no_unrelated_staging(repo_path, file_path)?;
    let mut args = vec!["--keep-index", "--include-untracked"];
    if let Some(msg) = message {
        args.push("-m");
        args.push(msg);
    }
    args.push("--");
    args.push(file_path);

    run_stash_push_command(repo_path, &args, true)
}

pub fn stash_untracked_file(
    repo_path: &str,
    file_path: &str,
    message: Option<&str>,
) -> Result<StashPushOutcome> {
    ensure_stash_path_has_no_unrelated_staging(repo_path, file_path)?;
    let mut args = vec!["--include-untracked"];
    if let Some(msg) = message {
        args.push("-m");
        args.push(msg);
    }
    args.push("--");
    args.push(file_path);

    run_stash_push_command(repo_path, &args, true)
}

/// Refuse qu'un stash ciblé capture aussi l'index d'autres fichiers.
fn ensure_stash_path_has_no_unrelated_staging(repo_path: &str, file_path: &str) -> Result<()> {
    let repo = Repository::discover(repo_path)?;
    let head = match repo.head() {
        Ok(head) => Some(head.peel_to_tree()?),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => None,
        Err(error) => return Err(error.into()),
    };
    let diff = repo.diff_tree_to_index(head.as_ref(), None, None)?;
    let selected = std::path::Path::new(file_path);
    for delta in diff.deltas() {
        if [delta.old_file(), delta.new_file()]
            .iter()
            .filter_map(|file| file.path())
            .any(|path| path != selected)
        {
            return Err(crate::error::GitSvError::Other(
                "Stash ciblé refusé : désindexez d'abord les autres fichiers pour éviter de les inclure dans ce stash".into(),
            ));
        }
    }
    Ok(())
}

/// Stash tous les changements non stagés (unstaged + untracked) en conservant l'index.
///
/// Une seule opération native conserve l'index, y compris pour les fichiers mixtes.
pub fn stash_unstaged_files(repo_path: &str, message: Option<&str>) -> Result<StashPushOutcome> {
    let repo = Repository::discover(repo_path)?;
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut options))?;
    let worktree_flags = git2::Status::WT_NEW
        | git2::Status::WT_MODIFIED
        | git2::Status::WT_DELETED
        | git2::Status::WT_RENAMED
        | git2::Status::WT_TYPECHANGE;
    if !statuses
        .iter()
        .any(|entry| entry.status().intersects(worktree_flags))
    {
        return Ok(StashPushOutcome::NoChanges);
    }
    let mut args = vec!["--keep-index", "--include-untracked"];
    if let Some(message) = message {
        args.extend(["-m", message]);
    }
    run_stash_push_command(repo_path, &args, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;

    #[cfg(unix)]
    #[test]
    fn stash_does_not_import_or_modify_files_through_a_replaced_parent_symlink() {
        for all in [false, true] {
            let (temp, mut repo) = create_test_repo();
            commit_file(&repo, "folder/file.txt", "base\n", "base");
            std::fs::remove_dir_all(temp.path().join("folder")).unwrap();
            let outside = tempfile::TempDir::new().unwrap();
            std::fs::write(outside.path().join("file.txt"), b"external secret\n").unwrap();
            std::os::unix::fs::symlink(outside.path(), temp.path().join("folder")).unwrap();
            let _ = if all {
                stash_unstaged_files(temp.path().to_str().unwrap(), None)
            } else {
                stash_file(temp.path().to_str().unwrap(), "folder/file.txt", None)
            };
            assert_eq!(
                std::fs::read(outside.path().join("file.txt")).unwrap(),
                b"external secret\n"
            );
            for stash in list_stashes(&mut repo).unwrap() {
                let tree = repo.find_commit(stash.oid).unwrap().tree().unwrap();
                if let Ok(entry) = tree.get_path(std::path::Path::new("folder/file.txt")) {
                    assert_ne!(
                        repo.find_blob(entry.id()).unwrap().content(),
                        b"external secret\n"
                    );
                }
            }
        }
    }

    #[test]
    fn stash_files_reject_non_utf8_paths_without_lossy_aliases() {
        let (_temp, repo) = create_test_repo();
        let base = commit_file(&repo, "base.txt", "base\n", "base");
        add_raw_index_file(&repo, b"invalid-\xff.txt", b"private content");
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.find_commit(base).unwrap();
        let signature = repo.signature().unwrap();
        let stash = repo
            .commit(None, &signature, &signature, "stash", &tree, &[&parent])
            .unwrap();
        assert!(matches!(
            stash_files(&repo, stash),
            Err(crate::error::GitSvError::UnsupportedPathEncoding)
        ));
    }

    #[test]
    fn successful_stash_pop_restores_bytes_and_drops_only_the_selected_stash() {
        let (temp, mut repo) = create_test_repo();
        repo.config()
            .unwrap()
            .set_bool("core.autocrlf", false)
            .unwrap();
        commit_file(&repo, "été 😀.txt", "base\r\n", "base");
        create_file(&repo, "été 😀.txt", "précédent\r\n");
        save_stash(&mut repo, Some("older")).unwrap();
        let older = repo.refname_to_id("refs/stash").unwrap();
        std::fs::write(temp.path().join("été 😀.txt"), b"binary\0\xff\r\n").unwrap();
        save_stash(&mut repo, Some("binary")).unwrap();
        pop_stash(&mut repo, 0).unwrap();
        assert_eq!(
            std::fs::read(temp.path().join("été 😀.txt")).unwrap(),
            b"binary\0\xff\r\n"
        );
        let remaining = list_stashes(&mut repo).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].oid, older);
    }

    #[test]
    fn stash_file_refuses_to_capture_an_unrelated_staged_file() {
        let (temp, mut repo) = create_test_repo();
        commit_file(&repo, "selected.txt", "base\n", "base");
        commit_file(&repo, "other.txt", "base\n", "other");
        create_file(&repo, "other.txt", "staged secret\n");
        crate::git::commit::stage_file(&repo, "other.txt").unwrap();
        create_file(&repo, "selected.txt", "selected change\n");
        let index_before = repo.index().unwrap().write_tree().unwrap();
        assert!(stash_file(
            temp.path().to_str().unwrap(),
            "selected.txt",
            Some("scoped")
        )
        .is_err());
        assert!(list_stashes(&mut repo).unwrap().is_empty());
        assert_eq!(
            std::fs::read(temp.path().join("selected.txt")).unwrap(),
            b"selected change\n"
        );
        assert_eq!(
            std::fs::read(temp.path().join("other.txt")).unwrap(),
            b"staged secret\n"
        );
        assert_eq!(repo.index().unwrap().write_tree().unwrap(), index_before);
    }

    #[test]
    fn stash_file_pathspec_is_literal_and_keeps_other_worktree_changes() {
        let (temp, mut repo) = create_test_repo();
        commit_file(&repo, "[a].txt", "selected base\n", "selected");
        commit_file(&repo, "a.txt", "other base\n", "other");
        create_file(&repo, "[a].txt", "selected change\n");
        create_file(&repo, "a.txt", "other change\n");
        stash_file(temp.path().to_str().unwrap(), "[a].txt", Some("--")).unwrap();
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes[0].files.len(), 1);
        assert_eq!(stashes[0].files[0].path, "[a].txt");
        assert_eq!(
            std::fs::read(temp.path().join("[a].txt")).unwrap(),
            b"selected base\n"
        );
        assert_eq!(
            std::fs::read(temp.path().join("a.txt")).unwrap(),
            b"other change\n"
        );
    }

    #[test]
    fn failed_stash_application_keeps_the_stash_and_unrelated_edits() {
        for pop in [true, false] {
            let (temp, mut repo) = create_test_repo();
            commit_file(&repo, "unrelated.txt", "base\n", "base");
            commit_file(&repo, "conflict.txt", "base\n", "conflict base");
            create_file(&repo, "conflict.txt", "stashed\n");
            save_stash(&mut repo, Some("keep on conflict")).unwrap();
            let saved = repo.refname_to_id("refs/stash").unwrap();
            commit_file(
                &repo,
                "conflict.txt",
                "current branch\n",
                "conflicting change",
            );
            create_file(&repo, "unrelated.txt", "local edit\n");
            let head_before = repo.head().unwrap().target();
            let result = if pop {
                pop_stash(&mut repo, 0)
            } else {
                apply_stash(&mut repo, 0)
            };
            assert_eq!(repo.refname_to_id("refs/stash").unwrap(), saved);
            assert_eq!(repo.head().unwrap().target(), head_before);
            assert_eq!(
                std::fs::read(temp.path().join("unrelated.txt")).unwrap(),
                b"local edit\n"
            );
            let conflict = std::fs::read_to_string(temp.path().join("conflict.txt")).unwrap();
            assert!(conflict.contains("current branch"));
            assert!(conflict.contains("stashed"));
            assert!(repo.index().unwrap().has_conflicts());
            assert!(result.is_err());
        }
    }

    #[test]
    fn unstaged_stash_does_not_pop_an_existing_stash_when_only_index_changed() {
        let (directory, mut repo) = create_test_repo();
        commit_file(&repo, "file.txt", "base\n", "base");
        create_file(&repo, "file.txt", "saved\n");
        save_stash(&mut repo, Some("previous stash")).unwrap();
        let previous = repo.refname_to_id("refs/stash").unwrap();
        create_file(&repo, "file.txt", "staged\n");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let staged_tree = index.write_tree().unwrap();

        assert_eq!(
            stash_unstaged_files(directory.path().to_str().unwrap(), None).unwrap(),
            StashPushOutcome::NoChanges
        );
        assert_eq!(repo.refname_to_id("refs/stash").unwrap(), previous);
        assert_eq!(repo.index().unwrap().write_tree().unwrap(), staged_tree);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("file.txt")).unwrap(),
            "staged\n"
        );
    }

    #[test]
    fn stash_lists_and_displays_previously_untracked_files() {
        let (directory, mut repo) = create_test_repo();
        commit_file(&repo, "tracked.txt", "base\n", "base");
        create_file(&repo, "untracked.txt", "saved content\n");
        stash_unstaged_files(directory.path().to_str().unwrap(), Some("untracked")).unwrap();
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0]
            .files
            .iter()
            .any(|file| file.path == "untracked.txt"));
        let diff = stash_file_diff(&repo, stashes[0].oid, "untracked.txt").unwrap();
        assert!(diff.iter().any(|line| line == "+saved content"));
    }

    #[test]
    fn stash_file_list_preserves_binary_modifications_and_deletions() {
        let (directory, mut repo) = create_test_repo();
        commit_file(&repo, "binary.dat", "old\0binary", "binary");
        commit_file(&repo, "deleted.txt", "removed", "text");
        create_file(&repo, "binary.dat", "new\0binary");
        std::fs::remove_file(directory.path().join("deleted.txt")).unwrap();
        save_stash(&mut repo, Some("binary and deletion")).unwrap();

        let stashes = list_stashes(&mut repo).unwrap();

        assert_eq!(stashes.len(), 1);
        assert_eq!(stashes[0].files.len(), 2);
        assert!(stashes[0]
            .files
            .iter()
            .any(|file| file.path == "binary.dat" && file.status == DiffStatus::Modified));
        assert!(stashes[0]
            .files
            .iter()
            .any(|file| file.path == "deleted.txt" && file.status == DiffStatus::Deleted));
    }

    #[test]
    fn test_extract_branch_from_message() {
        // Format WIP on <branch>: ...
        assert_eq!(
            extract_branch_from_message("WIP on main: abc123 modification"),
            Some("main".to_string())
        );
        // Format avec branche contenant un slash
        assert_eq!(
            extract_branch_from_message("WIP on feature/test: 123456 test"),
            Some("feature/test".to_string())
        );
        // Message sans format reconnaissable
        assert_eq!(
            extract_branch_from_message("Message sans format standard"),
            None
        );
        // Le format "On <branch>:" (sans WIP) utilise " on " avec espaces,
        // donc "On main:" ne correspond pas (c'est "On" pas " on ")
        assert_eq!(
            extract_branch_from_message("On main: Mon stash de test"),
            None
        );
    }

    #[test]
    fn test_save_stash() {
        let (_temp_dir, mut repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "initial", "Initial commit");

        // Modifier le fichier sans commit
        create_file(&repo, "test.txt", "modified");
        // Stage les modifications pour qu'elles soient stashées
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();

        // Sauvegarder le stash
        save_stash(&mut repo, Some("Mon stash de test")).unwrap();

        // Vérifier que le stash existe (le message contient "On main: " + notre message)
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].message.contains("Mon stash de test"));
        assert_eq!(stashes[0].index, 0);
    }

    #[test]
    fn test_list_stashes() {
        let (_temp_dir, mut repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "initial", "Initial commit");

        // Créer plusieurs stashes avec des modifications staged
        create_file(&repo, "file1.txt", "content1");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("file1.txt")).unwrap();
        index.write().unwrap();
        save_stash(&mut repo, Some("Stash 1")).unwrap();

        create_file(&repo, "file2.txt", "content2");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("file2.txt")).unwrap();
        index.write().unwrap();
        save_stash(&mut repo, Some("Stash 2")).unwrap();

        // Lister les stashes
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 2);
        // Le stash le plus récent a l'index 0
        assert!(stashes[0].message.contains("Stash 2"));
        assert!(stashes[1].message.contains("Stash 1"));
    }

    #[test]
    fn test_apply_stash() {
        let (_temp_dir, mut repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "initial", "Initial commit");

        // Créer et stash des modifications (fichier doit être staged)
        create_file(&repo, "new_file.txt", "new content");
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("new_file.txt"))
            .unwrap();
        index.write().unwrap();
        save_stash(&mut repo, Some("Test apply")).unwrap();

        // Le stash ne supprime pas le fichier, il le garde dans l'index git
        // Le fichier existe toujours physiquement
        let workdir = repo.workdir().unwrap().to_path_buf();

        // Appliquer le stash
        apply_stash(&mut repo, 0).unwrap();

        // Vérifier que le fichier existe
        assert!(workdir.join("new_file.txt").exists());

        // Le stash devrait toujours exister après apply
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
    }

    #[test]
    fn test_drop_stash() {
        let (_temp_dir, mut repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "initial", "Initial commit");

        // Créer un stash avec un fichier staged
        create_file(&repo, "temp.txt", "temp content");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("temp.txt")).unwrap();
        index.write().unwrap();
        save_stash(&mut repo, Some("To drop")).unwrap();

        // Vérifier qu'il existe
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);

        // Supprimer le stash
        drop_stash(&mut repo, 0).unwrap();

        // Vérifier qu'il n'existe plus
        let stashes = list_stashes(&mut repo).unwrap();
        assert!(stashes.is_empty());
    }

    #[test]
    fn test_stash_file_keeps_staged_part_for_same_path() {
        let (_temp_dir, mut repo) = create_test_repo();

        commit_file(&repo, "tracked.txt", "ligne 1\n", "Initial commit");

        create_file(&repo, "tracked.txt", "ligne 1\nversion staged\n");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("tracked.txt")).unwrap();
        index.write().unwrap();

        create_file(
            &repo,
            "tracked.txt",
            "ligne 1\nversion staged\nversion unstaged\n",
        );

        let outcome = stash_file(
            repo.workdir().unwrap().to_str().unwrap(),
            "tracked.txt",
            Some("stash fichier"),
        )
        .unwrap();

        assert_eq!(outcome, StashPushOutcome::Created);

        let tracked_status = {
            let statuses = repo.statuses(None).unwrap();
            statuses
                .iter()
                .find(|entry| entry.path().ok() == Some("tracked.txt"))
                .map(|entry| entry.status())
                .expect("tracked.txt devrait encore apparaitre dans le status")
        };

        assert!(tracked_status.contains(git2::Status::INDEX_MODIFIED));
        assert!(!tracked_status.contains(git2::Status::WT_MODIFIED));

        let content = std::fs::read_to_string(repo.workdir().unwrap().join("tracked.txt")).unwrap();
        assert_eq!(content, "ligne 1\nversion staged\n");

        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].message.contains("stash fichier"));
    }

    #[test]
    fn test_stash_file_limits_stash_to_selected_path() {
        let (_temp_dir, repo) = create_test_repo();

        commit_file(&repo, "selected.txt", "selected base\n", "Initial commit");
        commit_file(&repo, "other.txt", "other base\n", "Add other");

        create_file(&repo, "selected.txt", "selected modified\n");
        create_file(&repo, "other.txt", "other modified\n");

        let outcome = stash_file(
            repo.workdir().unwrap().to_str().unwrap(),
            "selected.txt",
            Some("stash selected"),
        )
        .unwrap();

        assert_eq!(outcome, StashPushOutcome::Created);
        assert!(repo.workdir().unwrap().join("other.txt").exists());

        let (selected_present, other_status) = {
            let statuses = repo.statuses(None).unwrap();
            let selected_present = statuses
                .iter()
                .any(|entry| entry.path().ok() == Some("selected.txt"));
            let other_status = statuses
                .iter()
                .find(|entry| entry.path().ok() == Some("other.txt"))
                .map(|entry| entry.status())
                .expect("other.txt devrait rester dans le working tree");
            (selected_present, other_status)
        };

        assert!(!selected_present);
        assert!(other_status.contains(git2::Status::WT_MODIFIED));
        assert_eq!(
            std::fs::read_to_string(repo.workdir().unwrap().join("selected.txt")).unwrap(),
            "selected base\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.workdir().unwrap().join("other.txt")).unwrap(),
            "other modified\n"
        );
    }

    #[test]
    fn test_stash_unstaged_files_preserves_index_and_stashes_untracked() {
        let (_temp_dir, mut repo) = create_test_repo();

        commit_file(&repo, "tracked.txt", "base\n", "Initial commit");

        create_file(&repo, "tracked.txt", "base\npartie staged\n");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("tracked.txt")).unwrap();
        index.write().unwrap();

        create_file(
            &repo,
            "tracked.txt",
            "base\npartie staged\npartie unstaged\n",
        );
        create_file(&repo, "extra.txt", "extra\n");

        let outcome = stash_unstaged_files(
            repo.workdir().unwrap().to_str().unwrap(),
            Some("stash unstaged"),
        )
        .unwrap();

        assert_eq!(outcome, StashPushOutcome::Created);

        let tracked_status = {
            let statuses = repo.statuses(None).unwrap();
            statuses
                .iter()
                .find(|entry| entry.path().ok() == Some("tracked.txt"))
                .map(|entry| entry.status())
                .expect("tracked.txt devrait rester stage")
        };

        assert!(tracked_status.contains(git2::Status::INDEX_MODIFIED));
        assert!(!tracked_status.contains(git2::Status::WT_MODIFIED));
        assert!(!repo.workdir().unwrap().join("extra.txt").exists());

        let content = std::fs::read_to_string(repo.workdir().unwrap().join("tracked.txt")).unwrap();
        assert_eq!(content, "base\npartie staged\n");

        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes.iter().any(|s| s.message.contains("stash unstaged")));
    }
}
