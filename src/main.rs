mod app;
mod error;
mod git;
mod ui;

use clap::{Parser, Subcommand};

use crate::app::App;
use crate::git::GitRepo;

#[derive(Parser)]
#[command(name = "git_sv")]
#[command(about = "Visualisez le graphe git de votre repo dans le terminal")]
#[command(version)]
struct Cli {
    /// Chemin du repository (défaut : répertoire courant)
    #[arg(short, long, default_value = ".")]
    path: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Affiche le log des commits (non-interactif)
    Log {
        /// Nombre maximum de commits à afficher
        #[arg(short = 'n', long, default_value = "20")]
        max_count: usize,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let repo = GitRepo::open(&cli.path)?;

    match cli.command {
        Some(Commands::Log { max_count }) => {
            // Mode non-interactif : affiche le log.
            print_log(&repo, max_count)?;
        }
        None => {
            // Mode par défaut : lance la TUI interactive.
            let mut app = App::new(repo, cli.path)?;
            app.run()?;
        }
    }

    Ok(())
}

/// Affiche le log des commits en mode non-interactif.
fn print_log(repo: &GitRepo, max_count: usize) -> anyhow::Result<()> {
    let commits = repo.log(max_count)?;

    for commit in &commits {
        let date = chrono::DateTime::from_timestamp(commit.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "???".to_string());

        println!(
            "\x1b[33m{}\x1b[0m {} \x1b[90m— {} ({})\x1b[0m",
            commit.short_hash(),
            commit.message,
            commit.author,
            date,
        );
    }

    Ok(())
}
