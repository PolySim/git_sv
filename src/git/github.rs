//! Lecture du statut de la pull request courante via GitHub CLI.

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const GH_TIMEOUT: Duration = Duration::from_secs(20);

/// Résultat de recherche d'une PR pour la branche courante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullRequestLookup {
    Found(GithubPullRequest),
    NotFound,
}

/// Statut de la pull request courante.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GithubPullRequest {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub review_decision: Option<String>,
    pub merge_state_status: Option<String>,
    pub url: String,
    pub additions: usize,
    pub deletions: usize,
    pub changed_files: usize,
    pub checks: CheckSummary,
}

impl GithubPullRequest {
    pub fn review_label(&self) -> &'static str {
        match self.review_decision.as_deref() {
            Some("APPROVED") => "approuvée",
            Some("CHANGES_REQUESTED") => "changements demandés",
            Some("REVIEW_REQUIRED") => "revue requise",
            _ => "sans décision",
        }
    }

    pub fn merge_label(&self) -> &'static str {
        match self.merge_state_status.as_deref() {
            Some("CLEAN") | Some("HAS_HOOKS") | Some("UNSTABLE") => "fusionnable",
            Some("BLOCKED") | Some("BEHIND") | Some("DIRTY") => "bloquée",
            _ => "état inconnu",
        }
    }
}

/// Agrégat des checks GitHub Actions et statuts externes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CheckSummary {
    pub passed: usize,
    pub failed: usize,
    pub pending: usize,
    pub skipped: usize,
}

impl CheckSummary {
    pub fn total(self) -> usize {
        self.passed + self.failed + self.pending + self.skipped
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPullRequest {
    number: u64,
    title: String,
    state: String,
    is_draft: bool,
    review_decision: Option<String>,
    merge_state_status: Option<String>,
    url: String,
    additions: usize,
    deletions: usize,
    changed_files: usize,
    status_check_rollup: Option<Vec<serde_json::Value>>,
}

/// Interroge `gh` pour la PR associée à la branche courante.
pub fn current_pull_request(repo_path: &Path) -> Result<PullRequestLookup, String> {
    let mut command = Command::new("gh");
    command
        .args([
            "pr",
            "view",
            "--json",
            "number,title,state,isDraft,reviewDecision,mergeStateStatus,url,additions,deletions,changedFiles,statusCheckRollup",
        ])
        .current_dir(repo_path)
        .env("GH_PROMPT_DISABLED", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = run_with_timeout(command)?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if is_no_pull_request_error(&error) {
            return Ok(PullRequestLookup::NotFound);
        }
        return Err(if error.is_empty() {
            format!("gh pr view a échoué avec {}", output.status)
        } else {
            error
        });
    }

    parse_pull_request(&output.stdout).map(PullRequestLookup::Found)
}

fn run_with_timeout(mut command: Command) -> Result<std::process::Output, String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("GitHub CLI indisponible: {error}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(|error| error.to_string()),
            Ok(None) if start.elapsed() < GH_TIMEOUT => {
                thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "GitHub CLI n'a pas répondu en {} secondes",
                    GH_TIMEOUT.as_secs()
                ));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn parse_pull_request(json: &[u8]) -> Result<GithubPullRequest, String> {
    let raw: RawPullRequest = serde_json::from_slice(json).map_err(|error| error.to_string())?;
    Ok(GithubPullRequest {
        number: raw.number,
        title: raw.title,
        state: raw.state,
        is_draft: raw.is_draft,
        review_decision: raw.review_decision,
        merge_state_status: raw.merge_state_status,
        url: raw.url,
        additions: raw.additions,
        deletions: raw.deletions,
        changed_files: raw.changed_files,
        checks: summarize_checks(raw.status_check_rollup.as_deref().unwrap_or_default()),
    })
}

fn is_no_pull_request_error(error: &str) -> bool {
    error
        .to_ascii_lowercase()
        .contains("no pull requests found")
}

fn summarize_checks(checks: &[serde_json::Value]) -> CheckSummary {
    let mut summary = CheckSummary::default();
    for check in checks {
        let conclusion = check
            .get("conclusion")
            .and_then(serde_json::Value::as_str)
            .or_else(|| check.get("state").and_then(serde_json::Value::as_str));
        match conclusion {
            Some("SUCCESS") | Some("NEUTRAL") => summary.passed += 1,
            Some("FAILURE")
            | Some("ERROR")
            | Some("TIMED_OUT")
            | Some("ACTION_REQUIRED")
            | Some("STARTUP_FAILURE") => summary.failed += 1,
            Some("SKIPPED") | Some("CANCELLED") => summary.skipped += 1,
            _ => summary.pending += 1,
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pull_request_and_checks() {
        let json = br#"{
            "number":42,
            "title":"Improve graph",
            "state":"OPEN",
            "isDraft":false,
            "reviewDecision":"APPROVED",
            "mergeStateStatus":"CLEAN",
            "url":"https://github.com/acme/repo/pull/42",
            "additions":120,
            "deletions":30,
            "changedFiles":8,
            "statusCheckRollup":[
                {"conclusion":"SUCCESS","status":"COMPLETED"},
                {"conclusion":"FAILURE","status":"COMPLETED"},
                {"conclusion":null,"status":"IN_PROGRESS"},
                {"state":"PENDING"}
            ]
        }"#;

        let pull_request = parse_pull_request(json).unwrap();

        assert_eq!(pull_request.number, 42);
        assert_eq!(pull_request.review_label(), "approuvée");
        assert_eq!(pull_request.merge_label(), "fusionnable");
        assert_eq!(
            pull_request.checks,
            CheckSummary {
                passed: 1,
                failed: 1,
                pending: 2,
                skipped: 0,
            }
        );
    }

    #[test]
    fn test_empty_checks_are_supported() {
        assert_eq!(summarize_checks(&[]).total(), 0);
        assert!(is_no_pull_request_error(
            "no pull requests found for branch main"
        ));
    }
}
