use git2::{Cred, CredentialType, RemoteCallbacks};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SshHostConfig {
    pub hostname: Option<String>,
    pub identity_file: Option<PathBuf>,
    pub user: Option<String>,
}

pub(super) fn parse_ssh_config() -> HashMap<String, SshHostConfig> {
    let config_path = dirs::home_dir()
        .map(|home| home.join(".ssh/config"))
        .or_else(|| {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .map(PathBuf::from)
                .map(|path| path.join(".ssh/config"))
                .ok()
        });

    let Some(config_path) = config_path else {
        return HashMap::new();
    };

    let Ok(content) = std::fs::read_to_string(config_path) else {
        return HashMap::new();
    };

    parse_ssh_config_content(&content, dirs::home_dir().as_deref())
}

pub(super) fn build_remote_callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        resolve_ssh_credentials(url, username_from_url, allowed_types)
    });
    callbacks
}

pub(super) fn resolve_remote_url(url: &str) -> String {
    let ssh_configs = parse_ssh_config();
    resolve_remote_url_with_configs(url, &ssh_configs)
}

fn parse_ssh_config_content(
    content: &str,
    home_dir: Option<&Path>,
) -> HashMap<String, SshHostConfig> {
    let mut configs = HashMap::new();
    let mut current_host: Option<String> = None;
    let mut current_config = SshHostConfig::default();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.to_lowercase().starts_with("host ") {
            if let Some(host) = current_host.take() {
                configs.insert(host, current_config.clone());
            }

            let host_pattern = line[5..].trim();
            if !host_pattern.contains('*') && !host_pattern.contains('?') {
                current_host = Some(host_pattern.to_string());
                current_config = SshHostConfig::default();
            }
        } else if line.to_lowercase().starts_with("hostname ") {
            if current_host.is_some() {
                current_config.hostname = Some(line[9..].trim().to_string());
            }
        } else if line.to_lowercase().starts_with("identityfile ") {
            if current_host.is_some() {
                current_config.identity_file =
                    Some(expand_tilde_with_home(line[13..].trim(), home_dir));
            }
        } else if line.to_lowercase().starts_with("user ") && current_host.is_some() {
            current_config.user = Some(line[5..].trim().to_string());
        }
    }

    if let Some(host) = current_host {
        configs.insert(host, current_config);
    }

    configs
}

fn expand_tilde(path: &str) -> PathBuf {
    expand_tilde_with_home(path, dirs::home_dir().as_deref())
}

fn expand_tilde_with_home(path: &str, home_dir: Option<&Path>) -> PathBuf {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = home_dir {
            return if path == "~" {
                home.to_path_buf()
            } else {
                home.join(&path[2..])
            };
        }
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            return if path == "~" {
                PathBuf::from(home)
            } else {
                PathBuf::from(home).join(&path[2..])
            };
        }
    }
    PathBuf::from(path)
}

fn extract_host_from_url(url: &str) -> Option<String> {
    if let Some(without_prefix) = url.strip_prefix("git@") {
        if let Some(colon_pos) = without_prefix.find(':') {
            return Some(without_prefix[..colon_pos].to_string());
        }
    } else if let Some(without_prefix) = url.strip_prefix("https://") {
        if let Some(slash_pos) = without_prefix.find('/') {
            return Some(without_prefix[..slash_pos].to_string());
        }
    } else if let Some(without_prefix) = url.strip_prefix("ssh://") {
        if let Some(at_pos) = without_prefix.find('@') {
            let after_at = &without_prefix[at_pos + 1..];
            if let Some(slash_pos) = after_at.find('/') {
                return Some(after_at[..slash_pos].to_string());
            }
        }
    }
    None
}

fn resolve_ssh_credentials(
    url: &str,
    username_from_url: Option<&str>,
    _allowed_types: CredentialType,
) -> std::result::Result<Cred, git2::Error> {
    let username = username_from_url.unwrap_or("git");
    let ssh_configs = parse_ssh_config();

    if let Some(host) = extract_host_from_url(url) {
        for config in ssh_configs.values() {
            if let Some(hostname) = &config.hostname {
                if hostname == &host {
                    if let Some(identity_path) = &config.identity_file {
                        let pubkey_path = identity_path.with_extension("pub");
                        return Cred::ssh_key(
                            config.user.as_deref().unwrap_or(username),
                            Some(&pubkey_path),
                            identity_path,
                            None,
                        );
                    }
                }
            }
        }

        if let Some(config) = ssh_configs.get(&host) {
            if let Some(identity_path) = &config.identity_file {
                let pubkey_path = identity_path.with_extension("pub");
                return Cred::ssh_key(
                    config.user.as_deref().unwrap_or(username),
                    Some(&pubkey_path),
                    identity_path,
                    None,
                );
            }
        }
    }

    if let Ok(cred) = Cred::ssh_key_from_agent(username) {
        return Ok(cred);
    }

    for key_name in ["id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"] {
        let key_path = expand_tilde(&format!("~/.ssh/{}", key_name));
        let pubkey_path = key_path.with_extension("pub");
        if key_path.exists() {
            return Cred::ssh_key(username, Some(&pubkey_path), &key_path, None);
        }
    }

    Err(git2::Error::from_str(&format!(
        "Erreur SSH: clé non trouvée pour '{}'. Vérifiez ~/.ssh/config",
        url
    )))
}

fn resolve_remote_url_with_configs(
    url: &str,
    ssh_configs: &HashMap<String, SshHostConfig>,
) -> String {
    if let Some(host) = extract_host_from_url(url) {
        if let Some(config) = ssh_configs.get(&host) {
            if let Some(real_hostname) = &config.hostname {
                return url.replacen(&host, real_hostname, 1);
            }
        }
    }

    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_ssh_config_content_extracts_hosts() {
        let content = r#"
            Host github-pro
                HostName github.com
                User git
                IdentityFile ~/.ssh/github_key

            Host gitlab-office
                HostName gitlab.com
                User deploy
                IdentityFile ~/.ssh/gitlab_key
        "#;

        let configs = parse_ssh_config_content(content, Some(Path::new("/Users/test")));

        assert_eq!(
            configs["github-pro"].hostname.as_deref(),
            Some("github.com")
        );
        assert_eq!(configs["github-pro"].user.as_deref(), Some("git"));
        assert_eq!(
            configs["github-pro"].identity_file.as_deref(),
            Some(Path::new("/Users/test/.ssh/github_key"))
        );
        assert_eq!(
            configs["gitlab-office"].hostname.as_deref(),
            Some("gitlab.com")
        );
    }

    #[test]
    fn test_resolve_remote_url_rewrites_alias() {
        let mut configs = HashMap::new();
        configs.insert(
            "github-pro".to_string(),
            SshHostConfig {
                hostname: Some("github.com".to_string()),
                identity_file: None,
                user: Some("git".to_string()),
            },
        );

        let resolved = resolve_remote_url_with_configs("git@github-pro:owner/repo.git", &configs);

        assert_eq!(resolved, "git@github.com:owner/repo.git");
    }

    #[test]
    fn test_resolve_remote_url_keeps_unknown_host() {
        let resolved =
            resolve_remote_url_with_configs("git@unknown-host:owner/repo.git", &HashMap::new());

        assert_eq!(resolved, "git@unknown-host:owner/repo.git");
    }
}
