//! `git_sv` — Visualisateur de graphe git en TUI.
//!
//! Point d'entrée CLI : parse les arguments avec clap,
//! ouvre le repository git, et lance l'interface interactive
//! ou le mode non-interactif selon les options.

mod app;
mod cli;
mod config;
mod error;
mod git;
mod handler;
mod i18n;
mod state;
mod terminal;
mod test_utils;
mod ui;
mod utils;
mod watcher;

use clap::{Parser, Subcommand};

use crate::app::App;
use crate::cli::{CliOptions, OutputFormat};
use crate::config::AppConfig;
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

    /// Affiche les thèmes disponibles et permet d'en choisir un
    #[command(visible_alias = "themes")]
    Theme {
        /// Thème à activer directement (dark, light ou solarized)
        #[arg(
            value_name = "THEME",
            value_parser = ["dark", "light", "solarized"]
        )]
        theme: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let mut config = AppConfig::load().unwrap_or_default();
    crate::i18n::set_language(config.language);

    let cli = Cli::parse();

    if let Some(Commands::Theme { theme }) = &cli.command {
        cli::theme(&mut config, theme.as_deref())?;
        return Ok(());
    }

    crate::ui::theme::init_theme(config.theme);
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
            let mut filter = GraphFilter {
                author,
                message,
                path: path_filter,
                ..GraphFilter::default()
            };

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
        Some(Commands::Theme { .. }) => unreachable!("La commande theme est traitée avant Git"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_theme_command_with_direct_value() {
        let cli = Cli::try_parse_from(["git_sv", "theme", "solarized"]).unwrap();

        assert!(matches!(
            cli.command,
            Some(Commands::Theme {
                theme: Some(ref value)
            }) if value == "solarized"
        ));
    }

    #[test]
    fn test_parse_themes_alias_without_value() {
        let cli = Cli::try_parse_from(["git_sv", "themes"]).unwrap();

        assert!(matches!(cli.command, Some(Commands::Theme { theme: None })));
    }
}
