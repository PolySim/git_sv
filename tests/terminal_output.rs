//! Vérifie les frontières entre données Git et commandes du terminal avec le vrai binaire.

#![cfg(unix)]

use std::path::Path;
use std::process::Command;

use git2::{Repository, RepositoryInitOptions, Signature};
use tempfile::TempDir;

fn run_cli(path: &Path, format: &str, command: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_git_sv"))
        .arg("--path")
        .arg(path)
        .args(["--format", format, command])
        .env("NO_HYPERLINK", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "CLI : {:?}", output.stderr);
    String::from_utf8(output.stdout).unwrap()
}

fn without_cli_colors(output: &str) -> String {
    let mut remaining = output;
    let mut result = String::new();
    while let Some(start) = remaining.find('\u{1b}') {
        result.push_str(&remaining[..start]);
        let escape = &remaining[start..];
        assert!(
            escape.starts_with("\u{1b}["),
            "commande terminal inattendue"
        );
        let end = escape.find('m').expect("couleur SGR complète");
        assert!(escape[2..end]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b';'));
        remaining = &escape[end + 1..];
    }
    result.push_str(remaining);
    assert!(!result
        .chars()
        .any(|character| character.is_control() && character != '\n'));
    assert!(!result.contains('\u{202e}'));
    result
}

#[test]
fn cli_escapes_terminal_commands_but_json_preserves_git_data() {
    let temp = TempDir::new().unwrap();
    let repo = Repository::init_opts(
        temp.path(),
        RepositoryInitOptions::new().initial_head("main"),
    )
    .unwrap();
    let message = "Résumé 漢字 e\u{301} 👩\u{200d}💻 \u{1b}]52;c;YWJj\u{7}\u{202e}";
    let author = "Zoé 漢字";
    let signature = Signature::now(author, "test@example.com").unwrap();
    let mut index = repo.index().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let oid = repo
        .commit(Some("HEAD"), &signature, &signature, message, &tree, &[])
        .unwrap();

    let controlled_path = "é\n\u{1b}]52;c;YWJj\u{7}\u{202e}.txt";
    let literal_path = "é\\n\\x1b]52;c;YWJj\\u{7}\\u{202e}.txt";
    let file_content = "contenu exact\r\n🙂\n";
    for path in [controlled_path, literal_path] {
        std::fs::write(temp.path().join(path), file_content).unwrap();
    }

    for format in ["plain", "human"] {
        let log = without_cli_colors(&run_cli(temp.path(), format, "log"));
        assert!(log.contains("Résumé 漢字 e\u{301} 👩\u{200d}💻"));
        assert!(log.contains("\\x1b]52;c;YWJj\\u{7}\\u{202e}"));
        let status = without_cli_colors(&run_cli(temp.path(), format, "status"));
        assert!(status.contains("é\\n\\x1b]52;c;YWJj\\u{7}\\u{202e}.txt"));
        assert!(status.contains("é\\\\n\\\\x1b]52;c;YWJj\\\\u{7}\\\\u{202e}.txt"));
    }

    let log: serde_json::Value =
        serde_json::from_str(&run_cli(temp.path(), "json", "log")).unwrap();
    assert_eq!(log[0]["message"], message);
    assert_eq!(log[0]["author"], author);
    let status: serde_json::Value =
        serde_json::from_str(&run_cli(temp.path(), "json", "status")).unwrap();
    for path in [controlled_path, literal_path] {
        assert!(status
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == path));
        assert_eq!(
            std::fs::read(temp.path().join(path)).unwrap(),
            file_content.as_bytes()
        );
    }
    assert_eq!(repo.head().unwrap().target(), Some(oid));
}

#[test]
fn missing_repository_errors_cannot_emit_terminal_commands() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("missing\u{1b}]52;c;YWJj\u{7}\u{202e}");
    let output = Command::new(env!("CARGO_BIN_EXE_git_sv"))
        .arg("--path")
        .arg(missing)
        .arg("status")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    let safe = without_cli_colors(&stderr);
    assert!(safe.contains("missing\\x1b]52;c;YWJj\\u{7}\\u{202e}"));
}

#[test]
fn argument_parser_errors_cannot_emit_terminal_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_git_sv"))
        .args(["--format", "bad\u{1b}]52;c;YWJj\u{7}\u{202e}", "log"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    let safe = without_cli_colors(&stderr);
    assert!(!safe.trim().is_empty());
}
