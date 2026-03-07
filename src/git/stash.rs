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
            oid: Oid::zero(),
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
        let files = stash_files(repo, oid).unwrap_or_default();
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

    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                let status = match delta.status() {
                    git2::Delta::Added => DiffStatus::Added,
                    git2::Delta::Modified => DiffStatus::Modified,
                    git2::Delta::Deleted => DiffStatus::Deleted,
                    git2::Delta::Renamed => DiffStatus::Renamed,
                    _ => return true,
                };
                files.push(StashFile {
                    path: path.to_string_lossy().to_string(),
                    status,
                });
            }
            true
        },
        None,
        None,
        None,
    )?;

    Ok(files)
}

/// Récupère le diff complet d'un fichier dans un stash.
pub fn stash_file_diff(repo: &Repository, stash_oid: Oid, file_path: &str) -> Result<Vec<String>> {
    let stash_commit = repo.find_commit(stash_oid)?;
    let stash_tree = stash_commit.tree()?;

    let parent = stash_commit.parent(0)?;
    let parent_tree = parent.tree()?;

    let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&stash_tree), None)?;

    let mut file_lines = Vec::new();
    let target_path = file_path.to_string();

    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        let is_target = delta
            .new_file()
            .path()
            .map(|p| p.to_string_lossy() == target_path)
            .unwrap_or(false)
            || delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy() == target_path)
                .unwrap_or(false);

        if is_target {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            file_lines.push(format!(
                "{}{}",
                prefix,
                String::from_utf8_lossy(line.content()).trim_end_matches('\n')
            ));
        }
        true
    })?;

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
    Ok(())
}

/// Applique et supprime le stash à l'index donné.
pub fn pop_stash(repo: &mut Repository, index: usize) -> Result<()> {
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_pop(index, Some(&mut opts))?;
    Ok(())
}

/// Supprime le stash à l'index donné sans l'appliquer.
pub fn drop_stash(repo: &mut Repository, index: usize) -> Result<()> {
    repo.stash_drop(index)?;
    Ok(())
}

/// Exécute `git stash push` via la CLI et interprète le résultat.
fn run_stash_push_command(repo_path: &str, args: &[&str]) -> Result<StashPushOutcome> {
    let mut cmd = Command::new("git");
    cmd.arg("stash").arg("push");
    cmd.args(args);
    cmd.current_dir(repo_path);

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined_output = format!("{}\n{}", stdout, stderr).to_lowercase();

    if combined_output.contains("no local changes to save") {
        return Ok(StashPushOutcome::NoChanges);
    }

    if !output.status.success() {
        return Err(crate::error::GitSvError::OperationFailed {
            operation: "stash_push",
            details: format!("git stash failed: {}", stderr.trim()),
        });
    }

    Ok(StashPushOutcome::Created)
}

/// Stash les changements non stagés d'un fichier spécifique en conservant l'index.
///
/// Utilise la CLI Git car libgit2 ne supporte pas ce workflow fin.
pub fn stash_file(
    repo_path: &str,
    file_path: &str,
    message: Option<&str>,
) -> Result<StashPushOutcome> {
    let mut args = vec!["--keep-index", "--include-untracked"];
    if let Some(msg) = message {
        args.push("-m");
        args.push(msg);
    }
    args.push("--");
    args.push(file_path);

    run_stash_push_command(repo_path, &args)
}

/// Stash tous les changements non stagés en conservant l'index.
pub fn stash_unstaged_files(repo_path: &str, message: Option<&str>) -> Result<StashPushOutcome> {
    let mut args = vec!["--keep-index", "--include-untracked"];
    if let Some(msg) = message {
        args.push("-m");
        args.push(msg);
    }

    run_stash_push_command(repo_path, &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;

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
                .find(|entry| entry.path() == Some("tracked.txt"))
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
                .any(|entry| entry.path() == Some("selected.txt"));
            let other_status = statuses
                .iter()
                .find(|entry| entry.path() == Some("other.txt"))
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
                .find(|entry| entry.path() == Some("tracked.txt"))
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
        assert!(stashes[0].message.contains("stash unstaged"));
    }
}
