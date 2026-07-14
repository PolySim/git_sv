//! Ouverture d'un diff dans l'outil configuré par Git.

use std::path::Path;
use std::process::Command;

use git2::Oid;

use crate::error::{GitSvError, Result};

/// Diff à déléguer à `git difftool` hors de la TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalDiffRequest {
    Commit { commit: Oid, path: String },
    WorkingTree { path: String, staged: bool },
}

impl ExternalDiffRequest {
    fn arguments(&self) -> Vec<String> {
        let mut arguments = vec!["difftool".to_string(), "--no-prompt".to_string()];
        match self {
            Self::Commit { commit, path } => {
                arguments.push(format!("{commit}^!"));
                arguments.push("--".to_string());
                arguments.push(path.clone());
            }
            Self::WorkingTree { path, staged } => {
                if *staged {
                    arguments.push("--cached".to_string());
                }
                arguments.push("--".to_string());
                arguments.push(path.clone());
            }
        }
        arguments
    }
}

/// Lance l'outil de diff configuré par `git config diff.tool`.
pub fn open(repo_path: &Path, request: &ExternalDiffRequest) -> Result<()> {
    let status = Command::new("git")
        .args(request.arguments())
        .current_dir(repo_path)
        .status()?;
    if status.success() {
        return Ok(());
    }
    Err(GitSvError::OperationFailed {
        operation: "git difftool",
        details: format!("l'outil externe s'est terminé avec {status}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn test_commit_diff_arguments_are_path_safe() {
        let request = ExternalDiffRequest::Commit {
            commit: Oid::zero(),
            path: "folder/file with spaces.rs".to_string(),
        };
        assert_eq!(
            request.arguments(),
            vec![
                "difftool",
                "--no-prompt",
                "0000000000000000000000000000000000000000^!",
                "--",
                "folder/file with spaces.rs",
            ]
        );
    }

    #[test]
    fn test_staged_diff_uses_cached_flag() {
        let request = ExternalDiffRequest::WorkingTree {
            path: "src/main.rs".to_string(),
            staged: true,
        };
        assert_eq!(
            request.arguments(),
            vec!["difftool", "--no-prompt", "--cached", "--", "src/main.rs"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_open_root_commit_with_configured_noninteractive_tool() {
        let (_directory, repo) = create_test_repo();
        let commit = commit_file(&repo, "file.txt", "content\n", "root");
        let mut config = repo.config().unwrap();
        config.set_str("diff.tool", "git-sv-test").unwrap();
        config.set_str("difftool.git-sv-test.cmd", "true").unwrap();

        open(
            repo.workdir().unwrap(),
            &ExternalDiffRequest::Commit {
                commit,
                path: "file.txt".to_string(),
            },
        )
        .unwrap();
    }
}
