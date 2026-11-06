//! Ouverture d'un diff dans l'outil configuré par Git.

use std::path::Path;
use std::process::Command;

use git2::{Oid, Repository};

use crate::error::{GitSvError, Result};

/// Diff à déléguer à `git difftool` hors de la TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalDiffRequest {
    Commit { commit: Oid, path: String },
    WorkingTree { path: String, staged: bool },
}

impl ExternalDiffRequest {
    fn arguments(&self, repo: &Repository) -> Result<Vec<String>> {
        let mut arguments = vec![
            "--literal-pathspecs".to_string(),
            "difftool".to_string(),
            "--no-prompt".to_string(),
        ];
        match self {
            Self::Commit { commit, path } => {
                let commit = repo.find_commit(*commit)?;
                let before = if commit.parent_count() > 0 {
                    commit.parent(0)?.tree_id()
                } else {
                    // Git a besoin d'un arbre vide réel, avec le format d'objet de ce dépôt.
                    repo.treebuilder(None)?.write()?
                };
                arguments.push(before.to_string());
                arguments.push(commit.tree_id().to_string());
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
        Ok(arguments)
    }
}

/// Lance l'outil de diff configuré par `git config diff.tool`.
pub fn open(repo_path: &Path, request: &ExternalDiffRequest) -> Result<()> {
    let repo = Repository::discover(repo_path)?;
    let status = super::helpers::clear_repository_environment(&mut Command::new("git"))
        .args(request.arguments(&repo)?)
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
        let (_temp, repo) = create_test_repo();
        let commit = commit_file(&repo, "folder/file with spaces.rs", "content", "initial");
        let request = ExternalDiffRequest::Commit {
            commit,
            path: "folder/file with spaces.rs".to_string(),
        };
        assert_eq!(
            request.arguments(&repo).unwrap(),
            vec![
                "--literal-pathspecs".to_string(),
                "difftool".to_string(),
                "--no-prompt".to_string(),
                repo.treebuilder(None).unwrap().write().unwrap().to_string(),
                repo.find_commit(commit).unwrap().tree_id().to_string(),
                "--".to_string(),
                "folder/file with spaces.rs".to_string(),
            ]
        );
    }

    #[test]
    fn test_staged_diff_uses_cached_flag() {
        let (_temp, repo) = create_test_repo();
        let request = ExternalDiffRequest::WorkingTree {
            path: "src/main.rs".to_string(),
            staged: true,
        };
        assert_eq!(
            request.arguments(&repo).unwrap(),
            vec![
                "--literal-pathspecs",
                "difftool",
                "--no-prompt",
                "--cached",
                "--",
                "src/main.rs"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_open_root_commit_with_configured_noninteractive_tool() {
        let (_directory, repo) = create_test_repo();
        let commit = commit_file(&repo, "file.txt", "content\n", "root");
        let mut config = repo.config().unwrap();
        config.set_str("diff.tool", "git-sv-test").unwrap();
        config
            .set_str(
                "difftool.git-sv-test.cmd",
                "cat \"$LOCAL\" > tool-before; cat \"$REMOTE\" > tool-after",
            )
            .unwrap();
        drop(config);

        open(
            repo.workdir().unwrap(),
            &ExternalDiffRequest::Commit {
                commit,
                path: "file.txt".to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            std::fs::read(repo.workdir().unwrap().join("tool-before")).unwrap(),
            b""
        );
        assert_eq!(
            std::fs::read(repo.workdir().unwrap().join("tool-after")).unwrap(),
            b"content\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn safety_external_diff_exports_only_the_selected_literal_path() {
        let (_temp, repo) = create_test_repo();
        commit_file(&repo, "a.txt", "private sibling\n", "sibling");
        commit_file(&repo, "[a].txt", "selected base\n", "selected");
        let root = repo.workdir().unwrap();
        std::fs::write(root.join("a.txt"), "private sibling changed\n").unwrap();
        std::fs::write(root.join("[a].txt"), "selected changed\n").unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("diff.tool", "git-sv-test").unwrap();
        config
            .set_str(
                "difftool.git-sv-test.cmd",
                "printf '%s\\0' \"$BASE\" >> tool-paths; cat \"$REMOTE\" >> tool-contents",
            )
            .unwrap();
        drop(config);
        open(
            root,
            &ExternalDiffRequest::WorkingTree {
                path: "[a].txt".into(),
                staged: false,
            },
        )
        .unwrap();
        assert_eq!(
            std::fs::read(root.join("tool-paths")).unwrap(),
            b"[a].txt\0"
        );
        assert_eq!(
            std::fs::read(root.join("tool-contents")).unwrap(),
            b"selected changed\n"
        );
    }
}
