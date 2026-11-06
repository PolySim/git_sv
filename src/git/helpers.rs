//! Fonctions utilitaires git (helpers internes).

use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::error::{GitSvError, Result};

/// Refuse les chemins qui deviendraient ambigus après une conversion avec remplacement.
pub(crate) fn path_as_utf8(path: &std::path::Path) -> Result<&str> {
    path.to_str().ok_or(GitSvError::UnsupportedPathEncoding)
}

/// Résout un fichier du working tree sans traverser de lien symbolique parent.
/// Le dernier composant peut être un lien, à examiner avec `symlink_metadata` ou `read_link`.
pub(crate) fn worktree_path(repo: &git2::Repository, path: &Path) -> Result<PathBuf> {
    let mut resolved = repo
        .workdir()
        .ok_or_else(|| GitSvError::Other("Working tree Git introuvable".into()))?
        .to_path_buf();
    let mut components = path.components().peekable();
    if components.peek().is_none() {
        return Err(GitSvError::Other("Chemin de fichier vide".into()));
    }
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else {
            return Err(GitSvError::Other("Chemin de fichier invalide".into()));
        };
        resolved.push(name);
        if components.peek().is_some() {
            match std::fs::symlink_metadata(&resolved) {
                Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
                // Une restauration Git peut recréer un dossier supprimé.
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
                Ok(_) => {
                    return Err(GitSvError::Other(
                        "Lecture refusée : le chemin traverse un lien ou un fichier".into(),
                    ))
                }
            }
        }
    }
    Ok(resolved)
}

/// Budget par flux : une commande trop bavarde échoue sans saturer la mémoire.
const MAX_COMMAND_OUTPUT: usize = 8 * 1024 * 1024;

/// Isole le dépôt choisi des variables exportées par un hook ou un autre dépôt.
/// Les variables d'authentification et d'éditeur restent disponibles.
pub(super) fn clear_repository_environment(command: &mut Command) -> &mut Command {
    // Variables locales recensées par `git rev-parse --local-env-vars`, plus le namespace.
    for name in [
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CONFIG",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
        "GIT_OBJECT_DIRECTORY",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_IMPLICIT_WORK_TREE",
        "GIT_GRAFT_FILE",
        "GIT_INDEX_FILE",
        "GIT_NO_REPLACE_OBJECTS",
        "GIT_REPLACE_REF_BASE",
        "GIT_PREFIX",
        "GIT_SHALLOW_FILE",
        "GIT_COMMON_DIR",
        "GIT_NAMESPACE",
    ] {
        command.env_remove(name);
    }
    command
}

/// Exécute une commande en drainant simultanément ses deux sorties.
/// Le délai couvre aussi la fermeture des pipes par les processus descendants.
pub(super) fn run_command_with_timeout(
    mut command: Command,
    operation: &'static str,
    timeout: Duration,
) -> Result<Output> {
    clear_repository_environment(&mut command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut process = CommandGuard {
        child: command.spawn()?,
        completed: false,
    };
    let (sender, receiver) = mpsc::channel();
    let stdout = process.child.stdout.take().expect("stdout redirigée");
    let stderr = process.child.stderr.take().expect("stderr redirigée");
    let stdout_sender = sender.clone();
    std::thread::Builder::new().spawn(move || {
        let _ = stdout_sender.send((true, read_command_output(stdout)));
    })?;
    std::thread::Builder::new().spawn(move || {
        let _ = sender.send((false, read_command_output(stderr)));
    })?;

    let start = Instant::now();
    let mut status = None;
    let mut stdout = None;
    let mut stderr = None;
    loop {
        for (is_stdout, result) in receiver.try_iter() {
            let bytes = result.map_err(|error| GitSvError::OperationFailed {
                operation,
                details: error.to_string(),
            })?;
            if is_stdout {
                stdout = Some(bytes);
            } else {
                stderr = Some(bytes);
            }
        }
        if status.is_none() {
            status = process.child.try_wait()?;
        }
        if let (Some(status), Some(stdout), Some(stderr)) =
            (status, stdout.as_mut(), stderr.as_mut())
        {
            process.completed = true;
            return Ok(Output {
                status,
                stdout: std::mem::take(stdout),
                stderr: std::mem::take(stderr),
            });
        }
        if start.elapsed() >= timeout {
            return Err(GitSvError::OperationFailed {
                operation,
                details: format!("délai dépassé après {} secondes", timeout.as_secs_f64()),
            });
        }
        std::thread::sleep(Duration::from_millis(10).min(timeout.saturating_sub(start.elapsed())));
    }
}

fn read_command_output(mut stream: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = match stream.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 {
            return Ok(bytes);
        }
        if bytes.len() + count > MAX_COMMAND_OUTPUT {
            return Err(io::Error::other("sortie de commande supérieure à 8 Mio"));
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

struct CommandGuard {
    child: Child,
    completed: bool,
}

impl Drop for CommandGuard {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        // Fermer aussi les descendants qui pourraient conserver les pipes ouverts.
        #[cfg(unix)]
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", self.child.id())])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        #[cfg(windows)]
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &self.child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Macro pour wrapper les opérations git avec contexte
#[macro_export]
macro_rules! git_op {
    ($op:expr, $operation_name:literal) => {
        $op.map_err(|e| GitSvError::OperationFailed {
            operation: $operation_name,
            details: e.to_string(),
        })
    };
}

/// Wrapper générique pour les opérations git qui retournent Result
#[allow(dead_code)] // Prevu pour usage futur.
pub fn with_error_context<T, E: std::fmt::Display>(
    result: std::result::Result<T, E>,
    operation: &'static str,
) -> Result<T> {
    result.map_err(|e| GitSvError::OperationFailed {
        operation,
        details: e.to_string(),
    })
}

/// Helper pour les opérations git optionnelles (retournent Option)
#[allow(dead_code)] // Prevu pour usage futur.
pub fn with_optional_context<T, E: std::fmt::Display>(
    result: std::result::Result<Option<T>, E>,
    operation: &'static str,
) -> Result<Option<T>> {
    result.map_err(|e| GitSvError::OperationFailed {
        operation,
        details: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_git_command_does_not_modify_a_repository_from_inherited_environment() {
        use crate::git::tests::test_utils::{commit_file, create_test_repo};

        let (_primary_dir, primary) = create_test_repo();
        let (_foreign_dir, foreign) = create_test_repo();
        for repo in [&primary, &foreign] {
            commit_file(repo, "file.txt", "original", "initial");
            std::fs::write(repo.workdir().unwrap().join("file.txt"), "modified").unwrap();
        }
        let foreign_index = std::fs::read(foreign.path().join("index")).unwrap();
        let mut command = Command::new("git");
        command
            .args(["add", "--", "file.txt"])
            .current_dir(primary.workdir().unwrap())
            .env("GIT_DIR", foreign.path())
            .env("GIT_WORK_TREE", foreign.workdir().unwrap())
            .env("GIT_INDEX_FILE", foreign.path().join("index"));
        let output = run_command_with_timeout(command, "test", Duration::from_secs(5)).unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        assert!(
            std::fs::read(foreign.path().join("index")).unwrap() == foreign_index,
            "L'index du dépôt extérieur a été modifié"
        );
        assert!(primary
            .status_file(Path::new("file.txt"))
            .unwrap()
            .contains(git2::Status::INDEX_MODIFIED));
    }

    #[test]
    fn command_output_has_a_byte_budget() {
        let bytes = vec![b'x'; MAX_COMMAND_OUTPUT + 1];
        assert!(read_command_output(bytes.as_slice()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn drains_both_pipes_before_waiting_for_exit() {
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "head -c 262144 /dev/zero; head -c 262144 /dev/zero >&2",
        ]);
        let output = run_command_with_timeout(command, "test", Duration::from_secs(5)).unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 262144);
        assert_eq!(output.stderr.len(), 262144);
    }

    #[cfg(unix)]
    #[test]
    fn timeout_covers_descendants_holding_pipes_open() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10 & exit 0"]);
        let start = Instant::now();
        let result = run_command_with_timeout(command, "test", Duration::from_millis(100));
        assert!(result.unwrap_err().to_string().contains("délai dépassé"));
        assert!(start.elapsed() < Duration::from_secs(3));
    }
}
