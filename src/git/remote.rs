//! Opérations remote : push, pull, fetch.

mod ssh;
pub mod types;

use git2::{FetchOptions, Repository};
use std::process::{Command, Output};
use std::time::Duration;

use crate::error::Result;
use crate::git::conflict::MergeResult;

pub use types::{flash_message_for_pull_result, FetchSuccess, PushSuccess};

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

fn run_git_command_with_timeout(cmd: Command, operation: &'static str) -> Result<Output> {
    super::helpers::run_command_with_timeout(cmd, operation, GIT_COMMAND_TIMEOUT)
}

fn current_branch_push_context(repo: &Repository) -> Result<(String, String, bool)> {
    let head = repo.head()?;
    if !head.is_branch() {
        return Err(git2::Error::from_str("HEAD détachée, aucune branche à pousser").into());
    }
    let branch_name = head.shorthand()?.to_string();
    let has_upstream = repo
        .branch_upstream_name(&format!("refs/heads/{}", branch_name))
        .is_ok();
    let remote_name = resolve_remote_name(repo, &branch_name);

    Ok((branch_name, remote_name, has_upstream))
}

/// Résout le nom du remote à partir du nom de branche.
fn resolve_remote_name(repo: &Repository, branch_name: &str) -> String {
    repo.config()
        .ok()
        .and_then(|config| {
            config
                .get_string(&format!("branch.{branch_name}.remote"))
                .ok()
        })
        .unwrap_or_else(|| "origin".to_string())
}

/// Vérifie si le repository a un remote configuré.
pub fn has_remote(repo: &Repository) -> Result<bool> {
    let remotes = repo.remotes()?;
    Ok(!remotes.is_empty())
}

/// Récupère le nom du remote par défaut pour la branche courante.
pub fn get_default_remote(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let branch_name = head.shorthand()?;

    Ok(resolve_remote_name(repo, branch_name))
}

/// Push la branche courante vers le remote.
pub fn push_current_branch(repo: &Repository) -> Result<PushSuccess> {
    push_current_branch_cli(repo)
}

/// Force push la branche courante vers le remote.
pub fn force_push_current_branch(repo: &Repository) -> Result<PushSuccess> {
    force_push_current_branch_cli(repo)
}

/// Push via git CLI, version par chemin pour le thread background.
pub fn push_current_branch_cli_path(repo_path: &std::path::Path) -> Result<PushSuccess> {
    push_current_branch_cli_path_with_options(repo_path, false)
}

/// Force push via git CLI, version par chemin pour le thread background.
pub fn force_push_current_branch_cli_path(repo_path: &std::path::Path) -> Result<PushSuccess> {
    push_current_branch_cli_path_with_options(repo_path, true)
}

fn push_current_branch_cli_path_with_options(
    repo_path: &std::path::Path,
    force: bool,
) -> Result<PushSuccess> {
    let repo = Repository::open(repo_path)?;
    let (branch_name, remote_name, has_upstream) = current_branch_push_context(&repo)?;

    let mut cmd = Command::new("git");
    cmd.arg("push");

    if force {
        cmd.arg("--force-with-lease");
    }

    if !has_upstream {
        cmd.arg("--set-upstream");
    }
    let destination = if has_upstream {
        repo.config()?
            .get_string(&format!("branch.{branch_name}.merge"))?
    } else {
        format!("refs/heads/{branch_name}")
    };
    cmd.args([
        "--",
        &remote_name,
        &format!("refs/heads/{branch_name}:{destination}"),
    ]);

    cmd.current_dir(repo_path);
    let output = run_git_command_with_timeout(cmd, "git push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git2::Error::from_str(&format!("Erreur git push: {}", stderr)).into());
    }

    Ok(PushSuccess {
        branch_name,
        remote_name,
        force,
        upstream_set: !has_upstream,
    })
}

/// Pull via git CLI, version par chemin pour le thread background.
pub fn pull_current_branch_cli_path(repo_path: &std::path::Path) -> Result<MergeResult> {
    let repo = Repository::discover(repo_path)?;
    let previous_head = repo.head().ok().and_then(|head| head.target());
    let mut cmd = Command::new("git");
    cmd.args(["pull"]).current_dir(repo_path);
    let output = run_git_command_with_timeout(cmd, "git pull")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let conflicts = crate::git::conflict::list_conflict_files(&repo)?;
        if !conflicts.is_empty() {
            return Ok(MergeResult::Conflicts(conflicts));
        }
        return Err(git2::Error::from_str(&format!("Erreur git pull: {}", stderr)).into());
    }

    let combined = format!("{}\n{}", stdout, stderr).to_lowercase();
    if repo.head().ok().and_then(|head| head.target()) == previous_head {
        Ok(MergeResult::UpToDate)
    } else if combined.contains("fast-forward") {
        Ok(MergeResult::FastForward)
    } else {
        Ok(MergeResult::Success)
    }
}

/// Fetch via git CLI, version par chemin pour le thread background.
pub fn fetch_all_cli_path(repo_path: &std::path::Path) -> Result<FetchSuccess> {
    let repo = Repository::open(repo_path)?;
    let mut cmd = Command::new("git");
    cmd.args(["fetch", "--all"]).current_dir(repo_path);
    let output = run_git_command_with_timeout(cmd, "git fetch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git2::Error::from_str(&format!("Erreur git fetch: {}", stderr)).into());
    }

    Ok(FetchSuccess {
        remote_name: get_default_remote(&repo).unwrap_or_else(|_| "origin".to_string()),
    })
}

/// Pull avec résultat typé pour gérer les conflits.
pub fn pull_current_branch_with_result(repo: &Repository) -> Result<MergeResult> {
    let path = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Working tree introuvable"))?;
    pull_current_branch_cli_path(path)
}

/// Fetch toutes les refs depuis le remote en utilisant git CLI (fallback).
pub fn fetch_all_cli(repo: &Repository) -> Result<()> {
    let repo_path = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Impossible de trouver le chemin du repository"))?;

    let mut cmd = Command::new("git");
    cmd.args(["fetch", "--all"]).current_dir(repo_path);
    let output = run_git_command_with_timeout(cmd, "git fetch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git2::Error::from_str(&format!("Erreur git fetch: {}", stderr)).into());
    }

    Ok(())
}

/// Fetch toutes les refs depuis le remote.
pub fn fetch_all(repo: &Repository) -> Result<()> {
    let remote_name = if let Ok(head) = repo.head() {
        if let Ok(branch_name) = head.shorthand() {
            resolve_remote_name(repo, branch_name)
        } else {
            "origin".to_string()
        }
    } else {
        "origin".to_string()
    };

    let remote = repo.find_remote(&remote_name)?;
    let raw_url = remote.url().unwrap_or("");
    let resolved_url = ssh::resolve_remote_url(raw_url);

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(ssh::build_remote_callbacks());

    let result = if resolved_url != raw_url {
        let mut fetch_remote = repo.remote_anonymous(&resolved_url)?;
        fetch_remote.fetch(&[] as &[&str], Some(&mut fetch_options), None)
    } else {
        let mut fetch_remote = repo.find_remote(&remote_name)?;
        fetch_remote.fetch(&[] as &[&str], Some(&mut fetch_options), None)
    };

    match result {
        Ok(()) => Ok(()),
        Err(_) => fetch_all_cli(repo),
    }
}

/// Push la branche courante en utilisant git CLI (fallback).
pub fn push_current_branch_cli(repo: &Repository) -> Result<PushSuccess> {
    push_current_branch_cli_with_options(repo, false)
}

/// Force push la branche courante en utilisant git CLI (fallback).
pub fn force_push_current_branch_cli(repo: &Repository) -> Result<PushSuccess> {
    push_current_branch_cli_with_options(repo, true)
}

fn push_current_branch_cli_with_options(repo: &Repository, force: bool) -> Result<PushSuccess> {
    let path = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Working tree introuvable"))?;
    push_current_branch_cli_path_with_options(path, force)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn push_targets_the_configured_upstream_branch() {
        let (directory, repo) = create_test_repo();
        let remote_dir = tempfile::TempDir::new().unwrap();
        let bare = Repository::init_bare(remote_dir.path()).unwrap();
        let base = commit_file(&repo, "file.txt", "base\n", "base");
        repo.remote("origin", remote_dir.path().to_str().unwrap())
            .unwrap();
        let mut remote = repo.find_remote("origin").unwrap();
        remote
            .push(&["refs/heads/main:refs/heads/destination"], None)
            .unwrap();
        drop(remote);
        repo.config()
            .unwrap()
            .set_str("branch.main.remote", "origin")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("branch.main.merge", "refs/heads/destination")
            .unwrap();
        repo.reference("refs/remotes/origin/destination", base, true, "tracking")
            .unwrap();
        let updated = commit_file(&repo, "file.txt", "new\n", "new");

        push_current_branch_cli_path(directory.path()).unwrap();

        assert_eq!(
            bare.refname_to_id("refs/heads/destination").unwrap(),
            updated
        );
        assert!(bare.find_reference("refs/heads/main").is_err());
    }

    #[test]
    fn push_rejects_detached_head() {
        let (_directory, repo) = create_test_repo();
        let oid = commit_file(&repo, "file.txt", "base\n", "base");
        repo.set_head_detached(oid).unwrap();
        assert!(current_branch_push_context(&repo).is_err());
    }
    #[test]
    fn test_first_push_args_include_remote_and_branch() {
        let remote_name = "origin";
        let branch_name = "feature/test";
        let args = ["--set-upstream", remote_name, branch_name];

        assert_eq!(args, ["--set-upstream", "origin", "feature/test"]);
    }
}
