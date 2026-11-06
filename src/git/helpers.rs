//! Fonctions utilitaires git (helpers internes).

use std::io::{self, Read};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::error::{GitSvError, Result};

/// Budget par flux : une commande trop bavarde échoue sans saturer la mémoire.
const MAX_COMMAND_OUTPUT: usize = 8 * 1024 * 1024;

/// Exécute une commande en drainant simultanément ses deux sorties.
/// Le délai couvre aussi la fermeture des pipes par les processus descendants.
pub(super) fn run_command_with_timeout(
    mut command: Command,
    operation: &'static str,
    timeout: Duration,
) -> Result<Output> {
    command
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
