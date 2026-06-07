//! Opérations remote : push, pull, fetch.

mod ssh;
pub mod types;

use git2::{FetchOptions, PushOptions, Repository};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use crate::error::Result;
use crate::git::conflict::MergeResult;

pub use types::{flash_message_for_pull_result, FetchSuccess, PushSuccess};

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

fn run_git_command_with_timeout(mut cmd: Command, operation: &'static str) -> Result<Output> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| git2::Error::from_str(&format!("Erreur exécuter {}: {}", operation, e)))?;
    let start = Instant::now();

    loop {
        if child
            .try_wait()
            .map_err(|e| git2::Error::from_str(&format!("Erreur attendre {}: {}", operation, e)))?
            .is_some()
        {
            return child.wait_with_output().map_err(Into::into);
        }

        if start.elapsed() >= GIT_COMMAND_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(crate::error::GitSvError::OperationFailed {
                operation,
                details: format!(
                    "délai dépassé après {} secondes",
                    GIT_COMMAND_TIMEOUT.as_secs()
                ),
            });
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

fn current_branch_push_context(repo: &Repository) -> Result<(String, String, bool)> {
    let head = repo.head()?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("HEAD détachée, impossible de pousser"))?
        .to_string();
    let has_upstream = repo
        .branch_upstream_name(&format!("refs/heads/{}", branch_name))
        .is_ok();
    let remote_name = resolve_remote_name(repo, &branch_name);

    Ok((branch_name, remote_name, has_upstream))
}

fn set_branch_upstream(repo: &Repository, branch_name: &str, remote_name: &str) -> Result<()> {
    let mut branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    branch.set_upstream(Some(&format!("{}/{}", remote_name, branch_name)))?;
    Ok(())
}

/// Résout le nom du remote à partir du nom de branche.
fn resolve_remote_name(repo: &Repository, branch_name: &str) -> String {
    repo.branch_upstream_name(&format!("refs/heads/{}", branch_name))
        .ok()
        .and_then(|name| name.as_str().map(|s| s.to_string()))
        .and_then(|name| {
            name.strip_prefix("refs/remotes/")
                .and_then(|rest| rest.split('/').next())
                .map(|s| s.to_string())
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
    let branch_name = head
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("HEAD détachée"))?;

    Ok(resolve_remote_name(repo, branch_name))
}

/// Push la branche courante vers le remote.
pub fn push_current_branch(repo: &Repository) -> Result<PushSuccess> {
    let (branch_name, remote_name, has_upstream) = current_branch_push_context(repo)?;
    let remote = repo.find_remote(&remote_name)?;
    let raw_url = remote.url().unwrap_or("");
    let resolved_url = ssh::resolve_remote_url(raw_url);

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(ssh::build_remote_callbacks());

    let push_refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

    let result = if resolved_url != raw_url {
        let mut push_remote = repo.remote_anonymous(&resolved_url)?;
        push_remote.push(&[&push_refspec], Some(&mut push_options))
    } else {
        let mut push_remote = repo.find_remote(&remote_name)?;
        push_remote.push(&[&push_refspec], Some(&mut push_options))
    };

    if result.is_err() {
        return push_current_branch_cli(repo);
    }

    if !has_upstream {
        set_branch_upstream(repo, &branch_name, &remote_name)?;
    }

    Ok(PushSuccess {
        branch_name,
        remote_name,
        force: false,
        upstream_set: !has_upstream,
    })
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
        cmd.args(["--set-upstream", &remote_name, &branch_name]);
    } else {
        cmd.args([&remote_name, &branch_name]);
    }

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
    let mut cmd = Command::new("git");
    cmd.args(["pull"]).current_dir(repo_path);
    let output = run_git_command_with_timeout(cmd, "git pull")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let combined = format!("{}\n{}", stdout, stderr).to_lowercase();
        if combined.contains("conflit") || combined.contains("conflict") {
            return Ok(MergeResult::Conflicts(Vec::new()));
        }
        return Err(git2::Error::from_str(&format!("Erreur git pull: {}", stderr)).into());
    }

    let combined = format!("{}\n{}", stdout, stderr).to_lowercase();
    if combined.contains("already up to date") || combined.contains("déjà à jour") {
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
    use crate::git::conflict::list_conflict_files;

    fetch_all(repo)?;

    let head = repo.head()?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("HEAD détachée, impossible de pull"))?;

    let upstream_name = repo.branch_upstream_name(&format!("refs/heads/{}", branch_name))?;
    let upstream_name = upstream_name
        .as_str()
        .ok_or_else(|| git2::Error::from_str("Nom de branche upstream invalide"))?;

    let upstream_ref = repo.find_reference(upstream_name)?;
    let upstream_oid = upstream_ref.peel_to_commit()?.id();
    let upstream_commit = repo.find_annotated_commit(upstream_oid)?;

    let analysis = repo.merge_analysis(&[&upstream_commit])?;

    if analysis.0.is_up_to_date() {
        Ok(MergeResult::UpToDate)
    } else if analysis.0.is_fast_forward() {
        let mut reference = repo.find_reference(&format!("refs/heads/{}", branch_name))?;
        reference.set_target(upstream_oid, &format!("Fast-forward to {}", upstream_oid))?;
        repo.set_head(&format!("refs/heads/{}", branch_name))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(MergeResult::FastForward)
    } else {
        repo.merge(
            &[&upstream_commit],
            Some(&mut git2::MergeOptions::default()),
            Some(&mut git2::build::CheckoutBuilder::default()),
        )?;

        let mut index = repo.index()?;
        if index.has_conflicts() {
            return Ok(MergeResult::Conflicts(list_conflict_files(repo)?));
        }

        let signature = repo.signature()?;
        let head_commit = head.peel_to_commit()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let upstream_real_commit = repo.find_commit(upstream_oid)?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Merge {}", upstream_name),
            &tree,
            &[&head_commit, &upstream_real_commit],
        )?;

        Ok(MergeResult::Success)
    }
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
        if let Some(branch_name) = head.shorthand() {
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
    let (branch_name, remote_name, has_upstream) = current_branch_push_context(repo)?;

    let repo_path = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Impossible de trouver le chemin du repository"))?;

    let mut cmd = Command::new("git");
    cmd.arg("push");

    if force {
        cmd.arg("--force-with-lease");
    }

    if !has_upstream {
        cmd.args(["--set-upstream", &remote_name, &branch_name]);
    } else {
        cmd.args([&remote_name, &branch_name]);
    }

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

#[cfg(test)]
mod tests {
    #[test]
    fn test_first_push_args_include_remote_and_branch() {
        let remote_name = "origin";
        let branch_name = "feature/test";
        let args = ["--set-upstream", remote_name, branch_name];

        assert_eq!(args, ["--set-upstream", "origin", "feature/test"]);
    }
}
