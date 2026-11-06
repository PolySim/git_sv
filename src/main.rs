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
    #[arg(short, long, default_value = "human", global = true, value_parser = ["human", "h", "plain", "p", "json", "j"])]
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
        #[arg(long, value_parser = parse_date_start)]
        since: Option<i64>,

        /// Date de fin (format: YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_end)]
        until: Option<i64>,
    },

    /// Liste les branches
    Branches,

    /// Affiche le status du working directory
    Status,

    /// Inspecte les hooks, la signature de HEAD et les sous-modules
    Inspect,

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

/// Ne modifie que le diagnostic final, jamais l'erreur ou les données manipulées dans l'application.
struct TerminalError(anyhow::Error);

impl std::fmt::Debug for TerminalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&crate::utils::terminal_text(&format!("{:?}", self.0)))
    }
}

fn main() -> std::result::Result<(), TerminalError> {
    match run() {
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::BrokenPipe) =>
        {
            Ok(())
        }
        result => result.map_err(TerminalError),
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut config = AppConfig::load()?;
    crate::i18n::set_language(config.language);

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
            if since.zip(until).is_some_and(|(start, end)| start > end) {
                anyhow::bail!("--since doit précéder --until");
            }
            let filter = GraphFilter {
                author,
                message,
                path: path_filter,
                date_from: since,
                date_to: until,
            };

            cli::log_filtered(&repo, max_count, &filter, &options)?;
        }
        Some(Commands::Branches) => {
            cli::branches(&repo, &options)?;
        }
        Some(Commands::Status) => {
            cli::status(&repo, &options)?;
        }
        Some(Commands::Inspect) => {
            cli::inspect(&repo, &options)?;
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
            let app = App::new_with_config(repo, options.path, &config)?;
            app.run()?;
        }
    }

    Ok(())
}

/// Parse une date UTC, avec une borne de fin incluant toute la journée.
fn parse_date(date_str: &str, end_of_day: bool) -> std::result::Result<i64, String> {
    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| format!("Date invalide '{date_str}', format attendu YYYY-MM-DD"))?;
    let time = if end_of_day { (23, 59, 59) } else { (0, 0, 0) };
    Ok(date
        .and_hms_opt(time.0, time.1, time.2)
        .unwrap()
        .and_utc()
        .timestamp())
}

fn parse_date_start(value: &str) -> std::result::Result<i64, String> {
    parse_date(value, false)
}

fn parse_date_end(value: &str) -> std::result::Result<i64, String> {
    parse_date(value, true)
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

    #[test]
    fn test_parse_inspect_command() {
        let cli = Cli::try_parse_from(["git_sv", "inspect"]).unwrap();

        assert!(matches!(cli.command, Some(Commands::Inspect)));
    }

    #[test]
    fn date_filters_reject_invalid_dates_and_include_the_last_day() {
        assert!(Cli::try_parse_from(["git_sv", "log", "--since", "2026-02-30"]).is_err());
        assert_eq!(
            parse_date_end("2026-11-06").unwrap() - parse_date_start("2026-11-06").unwrap(),
            86399
        );
    }

    #[test]
    fn global_options_work_after_the_subcommand_and_reject_unknown_formats() {
        let cli =
            Cli::try_parse_from(["git_sv", "--path", "repo", "graph", "--format", "json"]).unwrap();
        assert_eq!(cli.format, "json");
        assert_eq!(cli.path, "repo");
        assert!(Cli::try_parse_from(["git_sv", "--format", "typo", "log"]).is_err());
    }
}
