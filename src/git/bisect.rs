//! Pilotage de `git bisect` avec conservation de son état natif.

use std::process::Command;

use git2::{Oid, Repository};

use crate::error::{GitSvError, Result};

/// Indique si une session bisect est active.
pub fn is_active(repo: &Repository) -> bool {
    repo.path().join("BISECT_START").is_file()
}

/// Démarre un bisect entre un commit connu bon et un commit connu mauvais.
pub fn start(repo: &Repository, good: Oid, bad: Oid) -> Result<String> {
    let good = good.to_string();
    let bad = bad.to_string();
    run(repo, &["bisect", "start", bad.as_str(), good.as_str()])
}

/// Marque le commit courant comme bon.
pub fn mark_good(repo: &Repository) -> Result<String> {
    run(repo, &["bisect", "good"])
}

/// Marque le commit courant comme mauvais.
pub fn mark_bad(repo: &Repository) -> Result<String> {
    run(repo, &["bisect", "bad"])
}

/// Termine le bisect et restaure la branche initiale.
pub fn reset(repo: &Repository) -> Result<String> {
    run(repo, &["bisect", "reset"])
}

fn run(repo: &Repository, arguments: &[&str]) -> Result<String> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitSvError::Other("Working tree Git introuvable".into()))?;
    let output = Command::new("git")
        .args(arguments)
        .current_dir(workdir)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(GitSvError::OperationFailed {
            operation: "git bisect",
            details: if stderr.is_empty() { stdout } else { stderr },
        });
    }
    Ok(summarize_output(&stdout))
}

fn summarize_output(output: &str) -> String {
    output
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Bisect mis à jour")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn test_bisect_lifecycle() {
        let (_directory, repo) = create_test_repo();
        let good = commit_file(&repo, "file.txt", "0", "good");
        commit_file(&repo, "file.txt", "1", "one");
        commit_file(&repo, "file.txt", "2", "two");
        let bad = commit_file(&repo, "file.txt", "3", "bad");

        start(&repo, good, bad).unwrap();
        assert!(is_active(&repo));
        mark_bad(&repo).unwrap();
        assert!(is_active(&repo));
        reset(&repo).unwrap();
        assert!(!is_active(&repo));
        assert_eq!(repo.head().unwrap().target(), Some(bad));
    }
}
