//! CLI non-interactif pour git_sv.
//!
//! Fournit des commandes utilitaires pour scripts et usages rapides
//! sans lancer la TUI complète.

use crate::git::{branch::BranchInfo, commit::CommitInfo, GitRepo};
use crate::state::GraphFilter;
use anyhow::Result;
use serde::Serialize;
use std::io::Write;

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
        OutputFormat::Human => print_status_human(&entries)?,
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
        writeln!(handle, "\x1b[90mRemote branches:\x1b[0m")?;
        for branch in remote {
            writeln!(handle, "  {}", branch.name)?;
        }
    }

    Ok(())
}

fn print_status_human(entries: &[crate::git::repo::StatusEntry]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    if entries.is_empty() {
        writeln!(handle, "\x1b[32mWorking directory clean\x1b[0m")?;
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
        writeln!(handle, "\x1b[32mChanges to be committed:\x1b[0m")?;
        for path in &staged {
            writeln!(handle, "  \x1b[32m+ {}\x1b[0m", path)?;
        }
        writeln!(handle)?;
    }

    if !unstaged.is_empty() {
        writeln!(handle, "\x1b[33mChanges not staged for commit:\x1b[0m")?;
        for path in &unstaged {
            writeln!(handle, "  \x1b[33mM {}\x1b[0m", path)?;
        }
        writeln!(handle)?;
    }

    if !untracked.is_empty() {
        writeln!(handle, "\x1b[90mUntracked files:\x1b[0m")?;
        for path in &untracked {
            writeln!(handle, "  \x1b[90m? {}\x1b[0m", path)?;
        }
    }

    Ok(())
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

fn print_graph_json(_graph_rows: &[crate::git::graph::GraphRow]) -> Result<()> {
    // Pour l'instant, retourner un objet vide
    println!("{{}}");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fonctions utilitaires
// ═══════════════════════════════════════════════════════════════════════════════

fn format_timestamp(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "???".to_string())
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
}
