//! CLI non-interactif pour git_sv.
//!
//! Fournit des commandes utilitaires pour scripts et usages rapides
//! sans lancer la TUI complète.

use crate::config::{AppConfig, ThemeMode};
use crate::git::{branch::BranchInfo, commit::CommitInfo, GitRepo};
use crate::i18n::{text, text_owned};
use crate::state::GraphFilter;
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

/// Format de sortie pour les commandes CLI.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum OutputFormat {
    /// Format humain lisible (défaut)
    #[default]
    Human,
    /// Format JSON pour scripting
    Json,
    /// Format plain text (sans couleurs)
    Plain,
}

impl OutputFormat {
    /// Parse une chaîne en OutputFormat.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "human" | "h" => Some(Self::Human),
            "json" | "j" => Some(Self::Json),
            "plain" | "p" => Some(Self::Plain),
            _ => None,
        }
    }
}

/// Options communes pour les commandes CLI.
#[derive(Debug, Clone)]
pub struct CliOptions {
    pub format: OutputFormat,
    pub path: String,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            format: OutputFormat::Human,
            path: ".".to_string(),
        }
    }
}

/// Affiche le log filtré.
pub fn log_filtered(
    repo: &GitRepo,
    max_count: usize,
    filter: &GraphFilter,
    options: &CliOptions,
) -> Result<()> {
    let commits = repo.log_filtered(max_count, filter)?;

    match options.format {
        OutputFormat::Json => print_log_json(&commits)?,
        OutputFormat::Plain => print_log_plain(&commits)?,
        OutputFormat::Human => print_log_human(&commits)?,
    }

    Ok(())
}

/// Affiche la liste des branches.
pub fn branches(repo: &GitRepo, options: &CliOptions) -> Result<()> {
    let (local_branches, remote_branches) = crate::git::branch::list_all_branches(&repo.repo)?;

    match options.format {
        OutputFormat::Json => print_branches_json(&local_branches, &remote_branches)?,
        OutputFormat::Plain => print_branches_plain(&local_branches, &remote_branches)?,
        OutputFormat::Human => print_branches_human(&local_branches, &remote_branches)?,
    }

    Ok(())
}

/// Affiche le status du working directory.
pub fn status(repo: &GitRepo, options: &CliOptions) -> Result<()> {
    let entries = repo.status()?;

    match options.format {
        OutputFormat::Json => print_status_json(&entries)?,
        OutputFormat::Plain => print_status_plain(&entries)?,
        OutputFormat::Human => print_status_human(&entries, repo.repo.workdir())?,
    }

    Ok(())
}

/// Affiche les hooks, la signature de HEAD et les sous-modules.
pub fn inspect(repo: &GitRepo, options: &CliOptions) -> Result<()> {
    let commit = crate::git::insights::head_commit(&repo.repo)?;
    let insights = crate::git::insights::collect(&repo.repo, commit)?;

    if options.format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&insights)?);
        return Ok(());
    }

    let mut output = std::io::stdout().lock();
    writeln!(output, "Commit: {}", insights.commit)?;
    writeln!(output, "Signature: {}", insights.signature.summary())?;
    writeln!(
        output,
        "Hooks: {} actif(s) / {} configuré(s)",
        insights.enabled_hook_count(),
        insights.hooks.len()
    )?;
    for hook in &insights.hooks {
        let marker = if hook.enabled { "✓" } else { "○" };
        writeln!(output, "  {marker} {}", hook.name)?;
    }
    writeln!(
        output,
        "Sous-modules: {} · {} à vérifier",
        insights.submodules.len(),
        insights.dirty_submodule_count()
    )?;
    for submodule in &insights.submodules {
        writeln!(
            output,
            "  {} · {} · {}",
            submodule.path,
            submodule.revision.as_deref().unwrap_or("-------"),
            submodule.state.label()
        )?;
    }
    Ok(())
}

/// Recherche des commits.
pub fn search(repo: &GitRepo, query: &str, max_count: usize, options: &CliOptions) -> Result<()> {
    let commits = repo.search_commits(query, max_count)?;

    match options.format {
        OutputFormat::Json => print_log_json(&commits)?,
        OutputFormat::Plain => print_log_plain(&commits)?,
        OutputFormat::Human => print_log_human(&commits)?,
    }

    Ok(())
}

/// Affiche le graphe textuel.
pub fn graph(repo: &GitRepo, max_count: usize, options: &CliOptions) -> Result<()> {
    let graph_rows = repo.build_graph(max_count.min(50))?; // Limite pour éviter la surcharge

    match options.format {
        OutputFormat::Json => print_graph_json(&graph_rows)?,
        _ => print_graph_text(&graph_rows)?,
    }

    Ok(())
}

/// Affiche les thèmes disponibles et enregistre le choix de l'utilisateur.
pub fn theme(config: &mut AppConfig, requested: Option<&str>) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    let Some(selected) = select_theme(config.theme, requested, &mut input, &mut output)? else {
        writeln!(output, "{}", text("Aucun changement.", "No changes."))?;
        return Ok(());
    };

    config.theme = selected;
    let path = config.save()?;
    writeln!(
        output,
        "\x1b[32m✓ {}\x1b[0m {}",
        text("Thème activé :", "Theme enabled:"),
        selected.as_str()
    )?;
    writeln!(
        output,
        "  {} {}",
        text("Configuration :", "Configuration:"),
        path.display()
    )?;
    Ok(())
}

fn select_theme<R: BufRead, W: Write>(
    current: ThemeMode,
    requested: Option<&str>,
    input: &mut R,
    output: &mut W,
) -> Result<Option<ThemeMode>> {
    if let Some(requested) = requested {
        return parse_theme_choice(requested)
            .map(Some)
            .ok_or_else(|| invalid_theme_error(requested));
    }

    writeln!(
        output,
        "{}",
        text("Thèmes disponibles :", "Available themes:")
    )?;
    for (index, theme) in ThemeMode::ALL.iter().copied().enumerate() {
        let marker = if theme == current { "●" } else { " " };
        writeln!(
            output,
            " {marker} {}. {:<10} {}",
            index + 1,
            theme.as_str(),
            theme_description(theme)
        )?;
    }

    write!(
        output,
        "\n{} ",
        text(
            "Choisissez un thème [1-3, Entrée pour annuler] :",
            "Choose a theme [1-3, Enter to cancel]:"
        )
    )?;
    output.flush()?;

    let mut choice = String::new();
    if input.read_line(&mut choice)? == 0 || choice.trim().is_empty() {
        return Ok(None);
    }

    parse_theme_choice(&choice)
        .map(Some)
        .ok_or_else(|| invalid_theme_error(choice.trim()))
}

fn parse_theme_choice(value: &str) -> Option<ThemeMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "dark" => Some(ThemeMode::Dark),
        "2" | "light" => Some(ThemeMode::Light),
        "3" | "solarized" => Some(ThemeMode::Solarized),
        _ => None,
    }
}

fn theme_description(theme: ThemeMode) -> &'static str {
    match theme {
        ThemeMode::Dark => text(
            "Palette sombre à contraste élevé",
            "High-contrast dark palette",
        ),
        ThemeMode::Light => text("Palette claire neutre", "Neutral light palette"),
        ThemeMode::Solarized => text(
            "Palette ANSI héritée du terminal",
            "ANSI palette inherited from the terminal",
        ),
    }
}

fn invalid_theme_error(value: &str) -> anyhow::Error {
    anyhow!(
        "{} '{value}'. {}",
        text("Thème inconnu", "Unknown theme"),
        text(
            "Valeurs acceptées : dark, light, solarized",
            "Accepted values: dark, light, solarized"
        )
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fonctions d'affichage - Format humain
// ═══════════════════════════════════════════════════════════════════════════════

fn print_log_human(commits: &[CommitInfo]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    for commit in commits {
        let date = format_timestamp(commit.timestamp);

        writeln!(
            handle,
            "\x1b[33m{}\x1b[0m \x1b[1m{}\x1b[0m",
            &commit.oid.to_string()[..7],
            commit.message.lines().next().unwrap_or(""),
        )?;
        writeln!(handle, "  \x1b[90m{} | {}\x1b[0m", commit.author, date)?;
        writeln!(handle)?;
    }

    Ok(())
}

fn print_branches_human(local: &[BranchInfo], remote: &[BranchInfo]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    // Branche courante
    let current_branch = local.iter().find(|b| b.is_head);
    if let Some(branch) = current_branch {
        writeln!(handle, "\x1b[32m* {}\x1b[0m", branch.name)?;
    }

    // Autres branches locales
    for branch in local.iter().filter(|b| !b.is_head) {
        writeln!(handle, "  {}", branch.name)?;
    }

    // Branches distantes
    if !remote.is_empty() {
        writeln!(handle)?;
        writeln!(
            handle,
            "\x1b[90m{}\x1b[0m",
            text("Branches distantes:", "Remote branches:")
        )?;
        for branch in remote {
            writeln!(handle, "  {}", branch.name)?;
        }
    }

    Ok(())
}

fn print_status_human(
    entries: &[crate::git::repo::StatusEntry],
    workdir: Option<&Path>,
) -> Result<()> {
    let stdout = std::io::stdout();
    let hyperlinks = stdout.is_terminal()
        && std::env::var_os("NO_HYPERLINK").is_none()
        && std::env::var("TERM").map_or(true, |term| term != "dumb");
    let mut handle = stdout.lock();

    if entries.is_empty() {
        writeln!(
            handle,
            "\x1b[32m{}\x1b[0m",
            text("Repertoire de travail propre", "Working directory clean")
        )?;
        return Ok(());
    }

    // Grouper par statut
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    for entry in entries {
        let status = entry.status;
        if status.contains(git2::Status::INDEX_NEW)
            || status.contains(git2::Status::INDEX_MODIFIED)
            || status.contains(git2::Status::INDEX_DELETED)
        {
            staged.push(&entry.path);
        }
        if status.contains(git2::Status::WT_MODIFIED)
            || status.contains(git2::Status::WT_DELETED)
            || status.contains(git2::Status::WT_RENAMED)
        {
            unstaged.push(&entry.path);
        }
        if status.contains(git2::Status::WT_NEW) {
            untracked.push(&entry.path);
        }
    }

    if !staged.is_empty() {
        writeln!(
            handle,
            "\x1b[32m{}\x1b[0m",
            text("Changements a valider :", "Changes to be committed:")
        )?;
        for path in &staged {
            writeln!(
                handle,
                "  \x1b[32m+ {}\x1b[0m",
                terminal_file_link(workdir, path, hyperlinks)
            )?;
        }
        writeln!(handle)?;
    }

    if !unstaged.is_empty() {
        writeln!(
            handle,
            "\x1b[33m{}\x1b[0m",
            text(
                "Changements non indexes pour le commit :",
                "Changes not staged for commit:"
            )
        )?;
        for path in &unstaged {
            writeln!(
                handle,
                "  \x1b[33mM {}\x1b[0m",
                terminal_file_link(workdir, path, hyperlinks)
            )?;
        }
        writeln!(handle)?;
    }

    if !untracked.is_empty() {
        writeln!(
            handle,
            "\x1b[90m{}\x1b[0m",
            text("Fichiers non suivis :", "Untracked files:")
        )?;
        for path in &untracked {
            writeln!(
                handle,
                "  \x1b[90m? {}\x1b[0m",
                terminal_file_link(workdir, path, hyperlinks)
            )?;
        }
    }

    Ok(())
}

fn terminal_file_link(workdir: Option<&Path>, path: &str, enabled: bool) -> String {
    if !enabled {
        return path.to_string();
    }
    let Some(workdir) = workdir else {
        return path.to_string();
    };
    let absolute = workdir.join(path);
    let encoded = percent_encode_path(&absolute.to_string_lossy());
    format!("\x1b]8;;file://{encoded}\x1b\\{path}\x1b]8;;\x1b\\")
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~' | b':') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn print_graph_text(graph_rows: &[crate::git::graph::GraphRow]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    for row in graph_rows {
        // Simplification : afficher juste le commit
        let node = &row.node;
        let symbol = if node.parents.len() > 1 { "○" } else { "●" };
        writeln!(
            handle,
            "{} {} {}",
            symbol,
            &node.oid.to_string()[..7],
            node.message.lines().next().unwrap_or("")
        )?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fonctions d'affichage - Format plain
// ═══════════════════════════════════════════════════════════════════════════════

fn print_log_plain(commits: &[CommitInfo]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    for commit in commits {
        let date = format_timestamp(commit.timestamp);
        writeln!(
            handle,
            "{} {} {} {}",
            &commit.oid.to_string()[..7],
            commit.message.lines().next().unwrap_or(""),
            commit.author,
            date,
        )?;
    }

    Ok(())
}

fn print_branches_plain(local: &[BranchInfo], remote: &[BranchInfo]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    for branch in local {
        let prefix = if branch.is_head { "* " } else { "  " };
        writeln!(handle, "{}{}", prefix, branch.name)?;
    }

    for branch in remote {
        writeln!(handle, "  {}", branch.name)?;
    }

    Ok(())
}

fn print_status_plain(entries: &[crate::git::repo::StatusEntry]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    for entry in entries {
        let status_char = if entry.status.contains(git2::Status::INDEX_NEW) {
            "A"
        } else if entry.status.contains(git2::Status::INDEX_MODIFIED) {
            "M"
        } else if entry.status.contains(git2::Status::INDEX_DELETED) {
            "D"
        } else if entry.status.contains(git2::Status::WT_NEW) {
            "?"
        } else if entry.status.contains(git2::Status::WT_MODIFIED) {
            "M"
        } else {
            " "
        };
        writeln!(handle, "{} {}", status_char, entry.path)?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fonctions d'affichage - Format JSON
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
struct CommitJson {
    hash: String,
    message: String,
    author: String,
    email: String,
    timestamp: i64,
    date: String,
    parents: Vec<String>,
}

#[derive(Serialize)]
struct BranchJson {
    name: String,
    is_head: bool,
    is_remote: bool,
}

#[derive(Serialize)]
struct StatusJson {
    path: String,
    status: String,
    staged: bool,
}

#[derive(Serialize)]
struct GraphRefJson {
    name: String,
    kind: String,
}

#[derive(Serialize)]
struct GraphRowJson {
    hash: String,
    short_hash: String,
    message: String,
    author: String,
    timestamp: i64,
    parents: Vec<String>,
    column: usize,
    color_index: usize,
    is_merge: bool,
    refs: Vec<GraphRefJson>,
}

fn print_log_json(commits: &[CommitInfo]) -> Result<()> {
    let json_commits: Vec<CommitJson> = commits
        .iter()
        .map(|c| CommitJson {
            hash: c.oid.to_string(),
            message: c.message.clone(),
            author: c.author.clone(),
            email: c.email.clone(),
            timestamp: c.timestamp,
            date: format_timestamp(c.timestamp),
            parents: c.parents.iter().map(|p| p.to_string()).collect(),
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_commits)?);
    Ok(())
}

fn print_branches_json(local: &[BranchInfo], remote: &[BranchInfo]) -> Result<()> {
    let mut all_branches: Vec<BranchJson> = Vec::new();

    for branch in local {
        all_branches.push(BranchJson {
            name: branch.name.clone(),
            is_head: branch.is_head,
            is_remote: false,
        });
    }

    for branch in remote {
        all_branches.push(BranchJson {
            name: branch.name.clone(),
            is_head: false,
            is_remote: true,
        });
    }

    println!("{}", serde_json::to_string_pretty(&all_branches)?);
    Ok(())
}

fn print_status_json(entries: &[crate::git::repo::StatusEntry]) -> Result<()> {
    let json_entries: Vec<StatusJson> = entries
        .iter()
        .map(|e| StatusJson {
            path: e.path.clone(),
            status: format_status(&e.status),
            staged: e.is_staged(),
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_entries)?);
    Ok(())
}

fn print_graph_json(graph_rows: &[crate::git::graph::GraphRow]) -> Result<()> {
    let json_rows: Vec<GraphRowJson> = graph_rows
        .iter()
        .map(|row| GraphRowJson {
            hash: row.node.oid.to_string(),
            short_hash: row.node.short_hash(),
            message: row.node.message.clone(),
            author: row.node.author.clone(),
            timestamp: row.node.timestamp,
            parents: row
                .node
                .parents
                .iter()
                .map(|parent| parent.to_string())
                .collect(),
            column: row.node.column,
            color_index: row.node.color_index,
            is_merge: row.node.parents.len() > 1,
            refs: row
                .node
                .refs
                .iter()
                .map(|reference| GraphRefJson {
                    name: reference.name.clone(),
                    kind: match reference.ref_type {
                        crate::git::graph::RefType::LocalBranch => "local_branch",
                        crate::git::graph::RefType::RemoteBranch => "remote_branch",
                        crate::git::graph::RefType::Tag => "tag",
                        crate::git::graph::RefType::Head => "head",
                    }
                    .to_string(),
                })
                .collect(),
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_rows)?);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fonctions utilitaires
// ═══════════════════════════════════════════════════════════════════════════════

fn format_timestamp(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| text_owned("???", "???"))
}

fn format_status(status: &git2::Status) -> String {
    let mut result = String::new();

    if status.contains(git2::Status::INDEX_NEW) {
        result.push('A');
    } else if status.contains(git2::Status::INDEX_MODIFIED) {
        result.push('M');
    } else if status.contains(git2::Status::INDEX_DELETED) {
        result.push('D');
    } else if status.contains(git2::Status::INDEX_RENAMED) {
        result.push('R');
    } else {
        result.push(' ');
    }

    result.push(' ');

    if status.contains(git2::Status::WT_NEW) {
        result.push('?');
    } else if status.contains(git2::Status::WT_MODIFIED) {
        result.push('M');
    } else if status.contains(git2::Status::WT_DELETED) {
        result.push('D');
    } else if status.contains(git2::Status::WT_RENAMED) {
        result.push('R');
    } else {
        result.push(' ');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("human"), Some(OutputFormat::Human));
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("plain"), Some(OutputFormat::Plain));
        assert_eq!(OutputFormat::from_str("invalid"), None);
    }

    #[test]
    fn test_format_timestamp() {
        let ts = 1609459200; // 2021-01-01 00:00:00 UTC
        let formatted = format_timestamp(ts);
        assert!(formatted.contains("2021"));
    }

    #[test]
    fn test_format_status() {
        let status = git2::Status::INDEX_NEW | git2::Status::WT_MODIFIED;
        let formatted = format_status(&status);
        assert!(formatted.contains('A'));
        assert!(formatted.contains('M'));
    }

    #[test]
    fn test_terminal_file_link_encodes_path() {
        let link = terminal_file_link(Some(Path::new("/tmp/repo")), "a file#1.rs", true);

        assert!(link.contains("file:///tmp/repo/a%20file%231.rs"));
        assert!(link.contains("a file#1.rs"));
        assert_eq!(
            terminal_file_link(Some(Path::new("/tmp/repo")), "file.rs", false),
            "file.rs"
        );
    }

    #[test]
    fn test_select_theme_by_number_lists_all_themes() {
        let mut input = std::io::Cursor::new(b"3\n");
        let mut output = Vec::new();

        let selected = select_theme(ThemeMode::Dark, None, &mut input, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();

        assert_eq!(selected, Some(ThemeMode::Solarized));
        assert!(output.contains("dark"));
        assert!(output.contains("light"));
        assert!(output.contains("solarized"));
    }

    #[test]
    fn test_select_theme_accepts_direct_name() {
        let mut input = std::io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();

        let selected =
            select_theme(ThemeMode::Dark, Some("LIGHT"), &mut input, &mut output).unwrap();

        assert_eq!(selected, Some(ThemeMode::Light));
        assert!(output.is_empty());
    }

    #[test]
    fn test_select_theme_empty_input_cancels() {
        let mut input = std::io::Cursor::new(b"\n");
        let mut output = Vec::new();

        let selected = select_theme(ThemeMode::Dark, None, &mut input, &mut output).unwrap();

        assert_eq!(selected, None);
    }

    #[test]
    fn test_select_theme_rejects_unknown_value() {
        let mut input = std::io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();

        let error =
            select_theme(ThemeMode::Dark, Some("nord"), &mut input, &mut output).unwrap_err();

        assert!(error.to_string().contains("nord"));
    }
}
