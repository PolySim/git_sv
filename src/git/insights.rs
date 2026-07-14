//! Diagnostic local du dépôt : hooks, signatures et sous-modules.

use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use git2::{ErrorCode, Oid, Repository, SubmoduleIgnore};
use serde::Serialize;

use crate::error::{GitSvError, Result};

const SIGNATURE_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// Diagnostic synthétique d'un dépôt et d'un commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepositoryInsights {
    pub commit: String,
    pub signature: CommitSignatureStatus,
    pub hooks: Vec<HookInfo>,
    pub submodules: Vec<SubmoduleInfo>,
}

impl RepositoryInsights {
    /// Nombre de hooks exécutables.
    pub fn enabled_hook_count(&self) -> usize {
        self.hooks.iter().filter(|hook| hook.enabled).count()
    }

    /// Nombre de sous-modules nécessitant une attention.
    pub fn dirty_submodule_count(&self) -> usize {
        self.submodules
            .iter()
            .filter(|submodule| submodule.state != SubmoduleState::Clean)
            .count()
    }
}

/// État de la signature attachée au commit inspecté.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CommitSignatureStatus {
    Unsigned,
    Verified { kind: String },
    UnknownKey { kind: String },
    Invalid { kind: String },
    Present { kind: String },
}

impl CommitSignatureStatus {
    /// Résumé lisible destiné à la TUI et au mode texte.
    pub fn summary(&self) -> String {
        match self {
            Self::Unsigned => "non signée".to_string(),
            Self::Verified { kind } => format!("vérifiée ({kind})"),
            Self::UnknownKey { kind } => format!("clé inconnue ({kind})"),
            Self::Invalid { kind } => format!("invalide ({kind})"),
            Self::Present { kind } => format!("présente, non vérifiée ({kind})"),
        }
    }
}

/// Hook présent dans le dossier Git du dépôt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HookInfo {
    pub name: String,
    pub enabled: bool,
}

/// État utile d'un sous-module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmoduleState {
    Clean,
    Modified,
    Uninitialized,
}

impl SubmoduleState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Clean => "à jour",
            Self::Modified => "modifié",
            Self::Uninitialized => "non initialisé",
        }
    }
}

/// Informations locales d'un sous-module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubmoduleInfo {
    pub name: String,
    pub path: String,
    pub url: Option<String>,
    pub revision: Option<String>,
    pub state: SubmoduleState,
}

/// Collecte le diagnostic du dépôt pour un commit.
pub fn collect(repo: &Repository, commit: Oid) -> Result<RepositoryInsights> {
    Ok(RepositoryInsights {
        commit: commit.to_string(),
        signature: inspect_signature(repo, commit)?,
        hooks: list_hooks(repo)?,
        submodules: list_submodules(repo)?,
    })
}

/// Inspecte et, si possible, vérifie la signature cryptographique du commit.
pub fn inspect_signature(repo: &Repository, commit: Oid) -> Result<CommitSignatureStatus> {
    let signature = match repo.extract_signature(&commit, None) {
        Ok((signature, _)) => signature,
        Err(error) if error.code() == ErrorCode::NotFound => {
            return Ok(CommitSignatureStatus::Unsigned)
        }
        Err(error) => return Err(error.into()),
    };
    let kind = signature_kind(signature.as_str().unwrap_or_default()).to_string();

    Ok(verify_commit(repo, commit, kind))
}

fn signature_kind(signature: &str) -> &'static str {
    if signature.contains("BEGIN PGP SIGNATURE") {
        "OpenPGP"
    } else if signature.contains("BEGIN SSH SIGNATURE") {
        "SSH"
    } else if signature.contains("BEGIN SIGNED MESSAGE") || signature.contains("BEGIN CERTIFICATE")
    {
        "X.509"
    } else {
        "inconnue"
    }
}

fn verify_commit(repo: &Repository, commit: Oid, kind: String) -> CommitSignatureStatus {
    let directory = repo.workdir().unwrap_or_else(|| repo.path());
    let commit = commit.to_string();
    let mut command = Command::new("git");
    command
        .args(["verify-commit", "--raw", commit.as_str()])
        .current_dir(directory)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let Ok(mut child) = command.spawn() else {
        return CommitSignatureStatus::Present { kind };
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < SIGNATURE_CHECK_TIMEOUT => {
                thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return CommitSignatureStatus::Present { kind };
            }
        }
    }

    let Ok(output) = child.wait_with_output() else {
        return CommitSignatureStatus::Present { kind };
    };
    if output.status.success() {
        return CommitSignatureStatus::Verified { kind };
    }

    let diagnostic = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if diagnostic.contains("NO_PUBKEY")
        || diagnostic.contains("No public key")
        || diagnostic.contains("no public key")
    {
        CommitSignatureStatus::UnknownKey { kind }
    } else if diagnostic.contains("BADSIG")
        || diagnostic.contains("bad signature")
        || diagnostic.contains("BAD signature")
    {
        CommitSignatureStatus::Invalid { kind }
    } else {
        CommitSignatureStatus::Present { kind }
    }
}

/// Liste les hooks configurés en ignorant les exemples fournis par Git.
pub fn list_hooks(repo: &Repository) -> Result<Vec<HookInfo>> {
    let hooks_directory = repo.path().join("hooks");
    let entries = match fs::read_dir(&hooks_directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut hooks = Vec::new();
    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !metadata.is_file() || name.ends_with(".sample") || name.starts_with('.') {
            continue;
        }
        hooks.push(HookInfo {
            name,
            enabled: is_executable(&metadata),
        });
    }
    hooks.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(hooks)
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}

/// Liste les sous-modules et réduit les drapeaux libgit2 en états lisibles.
pub fn list_submodules(repo: &Repository) -> Result<Vec<SubmoduleInfo>> {
    let mut result = Vec::new();
    for submodule in repo.submodules()? {
        let name = submodule
            .name()
            .map(str::to_string)
            .unwrap_or_else(|| submodule.path().display().to_string());
        let status = repo.submodule_status(&name, SubmoduleIgnore::None)?;
        let state = if status.is_wd_uninitialized() || !status.is_in_wd() {
            SubmoduleState::Uninitialized
        } else if status.is_index_added()
            || status.is_index_deleted()
            || status.is_index_modified()
            || status.is_wd_added()
            || status.is_wd_deleted()
            || status.is_wd_modified()
            || status.is_wd_wd_modified()
            || status.is_wd_untracked()
        {
            SubmoduleState::Modified
        } else {
            SubmoduleState::Clean
        };
        result.push(SubmoduleInfo {
            name,
            path: submodule.path().display().to_string(),
            url: submodule.url().map(str::to_string),
            revision: submodule
                .workdir_id()
                .or_else(|| submodule.index_id())
                .map(|oid| {
                    let oid = oid.to_string();
                    oid[..7].to_string()
                }),
            state,
        });
    }
    result.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(result)
}

/// Vérifie qu'un identifiant de commit est disponible pour le diagnostic.
pub fn head_commit(repo: &Repository) -> Result<Oid> {
    repo.head()?
        .peel_to_commit()
        .map(|commit| commit.id())
        .map_err(|error| GitSvError::Other(format!("HEAD ne pointe pas vers un commit: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn test_unsigned_commit_and_empty_submodules() {
        let (_directory, repo) = create_test_repo();
        let oid = commit_file(&repo, "file.txt", "content", "unsigned");

        assert_eq!(
            inspect_signature(&repo, oid).unwrap(),
            CommitSignatureStatus::Unsigned
        );
        assert!(list_submodules(&repo).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn test_hooks_ignore_samples_and_report_executable_state() {
        use std::os::unix::fs::PermissionsExt;

        let (_directory, repo) = create_test_repo();
        let hooks = repo.path().join("hooks");
        fs::write(hooks.join("pre-commit"), "#!/bin/sh\n").unwrap();
        fs::set_permissions(hooks.join("pre-commit"), fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(hooks.join("commit-msg"), "#!/bin/sh\n").unwrap();
        fs::write(hooks.join("ignored.sample"), "#!/bin/sh\n").unwrap();

        assert_eq!(
            list_hooks(&repo).unwrap(),
            vec![
                HookInfo {
                    name: "commit-msg".to_string(),
                    enabled: false,
                },
                HookInfo {
                    name: "pre-commit".to_string(),
                    enabled: true,
                },
            ]
        );
    }

    #[test]
    fn test_signature_kind_detection() {
        assert_eq!(signature_kind("-----BEGIN PGP SIGNATURE-----"), "OpenPGP");
        assert_eq!(signature_kind("-----BEGIN SSH SIGNATURE-----"), "SSH");
        assert_eq!(signature_kind("unknown"), "inconnue");
    }
}
