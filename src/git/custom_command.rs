//! Exécution des commandes utilisateur hors du terminal TUI.

use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};

use crate::config::CustomCommandConfig;
use crate::error::Result;

/// Exécute une commande avec le shell utilisateur dans la racine du dépôt.
pub fn run(repo_path: &Path, definition: &CustomCommandConfig) -> Result<ExitStatus> {
    let mut command = shell_command(&definition.command);
    let status = command
        .current_dir(repo_path)
        .env("GIT_SV_REPO", repo_path)
        .status()?;

    if definition.pause {
        print!("\nAppuyez sur Entrée pour revenir à git_sv...");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
    }
    Ok(status)
}

#[cfg(unix)]
fn shell_command(command: &str) -> Command {
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
    let mut process = Command::new(shell);
    process.args(["-lc", command]);
    process
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut process = Command::new("cmd");
    process.args(["/C", command]);
    process
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn test_custom_command_runs_in_repository_directory() {
        let directory = tempfile::TempDir::new().unwrap();
        let marker = directory.path().join("marker.txt");
        let definition = CustomCommandConfig {
            name: "Test".to_string(),
            key: "alt+t".to_string(),
            command: "pwd > marker.txt".to_string(),
            confirm: false,
            pause: false,
        };

        let status = run(directory.path(), &definition).unwrap();

        assert!(status.success());
        assert_eq!(
            std::fs::read_to_string(marker).unwrap().trim(),
            directory.path().canonicalize().unwrap().to_string_lossy()
        );
    }
}
