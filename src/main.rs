//! `git_sv` — Visualisateur de graphe git en TUI.
//!
//! Point d'entrée CLI : parse les arguments avec clap,
//! ouvre le repository git, et lance l'interface interactive
//! ou le mode non-interactif selon les options.

mod app;
mod cli;
mod error;
mod error_display;
mod git;
mod handler;
mod state;
mod terminal;
mod test_utils;
mod ui;
mod utils;
mod watcher;

use clap::{Parser, Subcommand};

use crate::app::App;
use crate::cli::{CliOptions, OutputFormat};
use crate::git::GitRepo;
use crate::state::GraphFilter;

#[derive(Parser)]
#[command(name = "git_sv")]
#[command(about = "Visualisez le graphe git de votre repo dans le terminal")]
#[command(version)]
struct Cli {
    /// Chemin du repository (défaut : répertoire courant)
    #[arg(short, long, default_value = ".")]
    path: String,

    /// Format de sortie (human, plain, json)
    #[arg(short, long, default_value = "human")]
    format: String,

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

        /// Filtre par auteur
        #[arg(short, long)]
        author: Option<String>,

        /// Filtre par message (recherche dans le message)
        #[arg(short, long)]
        message: Option<String>,

        /// Filtre par chemin de fichier modifié
        #[arg(short, long)]
        path_filter: Option<String>,

        /// Date de début (format: YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Date de fin (format: YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
    },

    /// Liste les branches
    Branches,

    /// Affiche le status du working directory
    Status,

    /// Recherche des commits
    Search {
        /// Terme de recherche
        query: String,

        /// Nombre maximum de résultats
        #[arg(short = 'n', long, default_value = "20")]
        max_count: usize,
    },

    /// Affiche le graphe textuel
    Graph {
        /// Nombre maximum de commits à afficher
        #[arg(short = 'n', long, default_value = "20")]
        max_count: usize,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let repo = GitRepo::open(&cli.path)?;

    let format = OutputFormat::from_str(&cli.format).unwrap_or(OutputFormat::Human);
    let options = CliOptions {
        format,
        path: cli.path,
    };

    match cli.command {
        Some(Commands::Log {
            max_count,
            author,
            message,
            path_filter,
            since,
            until,
        }) => {
            // Mode non-interactif : affiche le log avec filtres
            let mut filter = GraphFilter::default();
            filter.author = author;
            filter.message = message;
            filter.path = path_filter;

            // Parse les dates si fournies
            if let Some(since_str) = since {
                filter.date_from = parse_date(&since_str);
            }
            if let Some(until_str) = until {
                filter.date_to = parse_date(&until_str);
            }

            cli::log_filtered(&repo, max_count, &filter, &options)?;
        }
        Some(Commands::Branches) => {
            cli::branches(&repo, &options)?;
        }
        Some(Commands::Status) => {
            cli::status(&repo, &options)?;
        }
        Some(Commands::Search { query, max_count }) => {
            cli::search(&repo, &query, max_count, &options)?;
        }
        Some(Commands::Graph { max_count }) => {
            cli::graph(&repo, max_count, &options)?;
        }
        None => {
            // Mode par défaut : lance la TUI interactive.
            let app = App::new(repo, options.path)?;
            app.run()?;
        }
    }

    Ok(())
}

/// Parse une date au format YYYY-MM-DD en timestamp.
fn parse_date(date_str: &str) -> Option<i64> {
    use chrono::NaiveDate;
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
}
