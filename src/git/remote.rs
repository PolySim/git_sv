use crate::error::Result;
use git2::{Cred, CredentialType, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration SSH extraite d'un Host dans ~/.ssh/config
#[derive(Debug, Clone, Default)]
struct SshHostConfig {
    hostname: Option<String>,
    identity_file: Option<PathBuf>,
    user: Option<String>,
}

/// Parse le fichier ~/.ssh/config et retourne une map Host -> Config
fn parse_ssh_config() -> HashMap<String, SshHostConfig> {
    let mut configs = HashMap::new();

    let config_path = dirs::home_dir().map(|h| h.join(".ssh/config")).or_else(|| {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .map(|p| p.join(".ssh/config"))
            .ok()
    });

    let config_path = match config_path {
        Some(p) => p,
        None => return configs,
    };

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return configs,
    };

    let mut current_host: Option<String> = None;
    let mut current_config = SshHostConfig::default();

    for line in content.lines() {
        let line = line.trim();

        // Ignorer les commentaires et lignes vides
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parser Host
        if line.to_lowercase().starts_with("host ") {
            // Sauvegarder le host précédent
            if let Some(host) = current_host.take() {
                configs.insert(host, current_config.clone());
            }

            let host_pattern = line[5..].trim();
            // Ignorer les wildcards globaux pour l'instant
            if !host_pattern.contains('*') && !host_pattern.contains('?') {
                current_host = Some(host_pattern.to_string());
                current_config = SshHostConfig::default();
            }
        }
        // Parser HostName
        else if line.to_lowercase().starts_with("hostname ") {
            if current_host.is_some() {
                current_config.hostname = Some(line[9..].trim().to_string());
            }
        }
        // Parser IdentityFile
        else if line.to_lowercase().starts_with("identityfile ") {
            if current_host.is_some() {
                let path = line[13..].trim();
                let expanded_path = expand_tilde(path);
                current_config.identity_file = Some(expanded_path);
            }
        }
        // Parser User
        else if line.to_lowercase().starts_with("user ") {
            if current_host.is_some() {
                current_config.user = Some(line[5..].trim().to_string());
            }
        }
    }

    // Sauvegarder le dernier host
    if let Some(host) = current_host {
        configs.insert(host, current_config);
    }

    configs
}

/// Remplace ~ par le répertoire home
fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = dirs::home_dir() {
            if path == "~" {
                return home;
            }
            return home.join(&path[2..]);
        }
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            if path == "~" {
                return PathBuf::from(home);
            }
            return PathBuf::from(home).join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

/// Extrait le hostname d'une URL SSH (ex: "git@github-pro:user/repo.git" -> "github-pro")
fn extract_host_from_url(url: &str) -> Option<String> {
    // Format SSH: git@hostname:user/repo.git
    if url.starts_with("git@") {
        let without_prefix = &url[4..];
        if let Some(colon_pos) = without_prefix.find(':') {
            return Some(without_prefix[..colon_pos].to_string());
        }
    }
    // Format HTTPS: https://hostname/user/repo.git
    else if url.starts_with("https://") {
        let without_prefix = &url[8..];
        if let Some(slash_pos) = without_prefix.find('/') {
            return Some(without_prefix[..slash_pos].to_string());
        }
    }
    // Format SSH avec schema: ssh://git@hostname/user/repo.git
    else if url.starts_with("ssh://") {
        let without_prefix = &url[6..];
        if let Some(at_pos) = without_prefix.find('@') {
            let after_at = &without_prefix[at_pos + 1..];
            if let Some(slash_pos) = after_at.find('/') {
                return Some(after_at[..slash_pos].to_string());
            }
        }
    }
    None
}

/// Résout les credentials SSH avec stratégie multi-niveaux
fn resolve_ssh_credentials(
    url: &str,
    username_from_url: Option<&str>,
    _allowed_types: CredentialType,
) -> std::result::Result<Cred, git2::Error> {
    let username = username_from_url.unwrap_or("git");

    // Étape 1: Parser la config SSH
    let ssh_configs = parse_ssh_config();

    // Étape 2: Extraire le hostname de l'URL
    if let Some(host) = extract_host_from_url(url) {
        eprintln!("[DEBUG] Looking for SSH config for host: {}", host);

        // Chercher D'ABORD la config par HostName (priorité aux alias)
        // Ex: URL = github.com, config = Host github-pro HostName github.com
        eprintln!("[DEBUG] Searching by HostName for: {}", host);
        for (alias, config) in &ssh_configs {
            if let Some(hostname) = &config.hostname {
                if hostname == &host {
                    eprintln!(
                        "[DEBUG] Found match: alias '{}' has HostName '{}'",
                        alias, hostname
                    );
                    if let Some(identity_path) = &config.identity_file {
                        eprintln!(
                            "[DEBUG] Using IdentityFile from alias '{}': {:?}",
                            alias, identity_path
                        );
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

        // Ensuite chercher la config directe pour ce host (fallback)
        if let Some(config) = ssh_configs.get(&host) {
            eprintln!("[DEBUG] Found direct config for host: {}", host);
            if let Some(identity_path) = &config.identity_file {
                eprintln!("[DEBUG] Using IdentityFile: {:?}", identity_path);
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

    // Étape 3: Fallback vers l'agent SSH
    eprintln!("[DEBUG] Falling back to SSH agent");
    if let Ok(cred) = Cred::ssh_key_from_agent(username) {
        return Ok(cred);
    }

    // Étape 4: Fallback vers les clés par défaut
    eprintln!("[DEBUG] Falling back to default keys");
    let default_keys = ["id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"];
    for key_name in &default_keys {
        let key_path = expand_tilde(&format!("~/.ssh/{}", key_name));
        let pubkey_path = key_path.with_extension("pub");

        if key_path.exists() {
            eprintln!("[DEBUG] Using default key: {:?}", key_path);
            return Cred::ssh_key(username, Some(&pubkey_path), &key_path, None);
        }
    }

    // Échec - retourner une erreur descriptive
    Err(git2::Error::from_str(&format!(
        "Erreur SSH: clé non trouvée pour '{}'. Vérifiez ~/.ssh/config",
        url
    )))
}

/// Construit les RemoteCallbacks avec la résolution SSH améliorée
fn build_remote_callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        eprintln!(
            "[DEBUG] SSH credentials callback - URL: {}, username: {:?}, allowed_types: {:?}",
            url, username_from_url, allowed_types
        );
        let result = resolve_ssh_credentials(url, username_from_url, allowed_types);
        eprintln!("[DEBUG] SSH credentials result: {:?}", result.is_ok());
        result
    });
    callbacks
}

/// Résout une URL SSH en remplaçant l'alias par le vrai hostname défini dans ~/.ssh/config
fn resolve_remote_url(url: &str) -> String {
    // Parser la config SSH
    let ssh_configs = parse_ssh_config();

    // Extraire le hostname de l'URL
    if let Some(host) = extract_host_from_url(url) {
        // Chercher si ce host a un HostName défini dans la config
        if let Some(config) = ssh_configs.get(&host) {
            if let Some(real_hostname) = &config.hostname {
                // Remplacer l'alias par le vrai hostname dans l'URL
                return url.replacen(&host, real_hostname, 1);
            }
        }
    }

    // Si pas d'alias trouvé, retourner l'URL telle quelle
    url.to_string()
}

/// Push la branche courante vers le remote.
/// Retourne un message décrivant l'action effectuée.
pub fn push_current_branch(repo: &Repository) -> Result<String> {
    // Récupérer la branche courante
    let head = repo.head()?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("HEAD détachée, impossible de pousser"))?;

    // Vérifier si la branche a un upstream configuré
    let has_upstream = repo
        .branch_upstream_name(&format!("refs/heads/{}", branch_name))
        .is_ok();

    // Récupérer le nom du remote (fallback vers "origin")
    let remote_name = repo
        .branch_upstream_name(&format!("refs/heads/{}", branch_name))
        .ok()
        .and_then(|name| name.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "origin".to_string());

    // Extraire juste le nom du remote (avant le premier '/')
    let remote_name = remote_name
        .split('/')
        .next()
        .unwrap_or("origin")
        .to_string();

    // Récupérer le remote configuré
    let remote = repo.find_remote(&remote_name)?;
    let remote_url = remote.url().unwrap_or("");
    eprintln!("[DEBUG] Original remote URL: {}", remote_url);

    // Résoudre l'URL (remplacer l'alias SSH par le vrai hostname)
    let resolved_url = resolve_remote_url(remote_url);
    eprintln!("[DEBUG] Resolved remote URL: {}", resolved_url);

    // Créer un remote temporaire avec l'URL résolue
    let temp_remote_name = format!("__temp_remote_{}", std::process::id());

    // Supprimer le remote temporaire s'il existe déjà
    let _ = repo.remote_delete(&temp_remote_name);

    // Créer le remote avec l'URL résolue
    let mut resolved_remote = repo.remote(&temp_remote_name, &resolved_url)?;

    // Options de push avec callbacks SSH améliorés
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(build_remote_callbacks());

    // Configurer le push avec --set-upstream si pas d'upstream configuré
    if !has_upstream {
        push_options.remote_push_options(&["--set-upstream"]);
    }

    // Pousser la branche courante
    let push_refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    let result = resolved_remote.push(&[&push_refspec], Some(&mut push_options));

    // Nettoyer le remote temporaire
    drop(resolved_remote);
    let _ = repo.remote_delete(&temp_remote_name);

    result?;

    // Retourner un message descriptif
    if has_upstream {
        Ok(format!("Push de '{}' vers {}", branch_name, remote_name))
    } else {
        Ok(format!(
            "Push de '{}' vers {}/{} (upstream configuré)",
            branch_name, remote_name, branch_name
        ))
    }
}

/// Pull (fetch + merge) depuis le remote.
pub fn pull_current_branch(repo: &Repository) -> Result<()> {
    // D'abord, faire un fetch
    fetch_all(repo)?;

    // Récupérer la branche courante
    let head = repo.head()?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("HEAD détachée, impossible de pull"))?;

    // Récupérer le nom complet de la branche upstream
    let upstream_name = repo.branch_upstream_name(&format!("refs/heads/{}", branch_name))?;
    let upstream_name = upstream_name
        .as_str()
        .ok_or_else(|| git2::Error::from_str("Nom de branche upstream invalide"))?;

    // Trouver le commit de l'upstream et créer un AnnotatedCommit
    let upstream_ref = repo.find_reference(upstream_name)?;
    let upstream_oid = upstream_ref.peel_to_commit()?.id();
    let upstream_commit = repo.find_annotated_commit(upstream_oid)?;

    // Merge avec fast-forward si possible
    let analysis = repo.merge_analysis(&[&upstream_commit])?;

    if analysis.0.is_up_to_date() {
        // Déjà à jour
        Ok(())
    } else if analysis.0.is_fast_forward() {
        // Fast-forward possible
        let mut reference = repo.find_reference(&format!("refs/heads/{}", branch_name))?;
        reference.set_target(upstream_oid, &format!("Fast-forward to {}", upstream_oid))?;
        repo.set_head(&format!("refs/heads/{}", branch_name))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    } else {
        // Merge nécessaire (conflits possibles)
        repo.merge(
            &[&upstream_commit],
            Some(&mut git2::MergeOptions::default()),
            Some(&mut git2::build::CheckoutBuilder::default()),
        )?;

        // Vérifier s'il y a des conflits
        let mut index = repo.index()?;
        if index.has_conflicts() {
            return Err(git2::Error::from_str("Conflits détectés lors du pull").into());
        }

        // Créer le commit de merge
        let signature = repo.signature()?;
        let head_commit = head.peel_to_commit()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let upstream_real_commit = repo.find_commit(upstream_oid)?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Merge {}", upstream_name),
            &tree,
            &[&head_commit, &upstream_real_commit],
        )?;

        Ok(())
    }
}

/// Fetch toutes les refs depuis le remote.
pub fn fetch_all(repo: &Repository) -> Result<()> {
    // Récupérer le remote par défaut (généralement "origin")
    let remote_name = "origin";
    let remote = repo.find_remote(remote_name)?;
    let remote_url = remote.url().unwrap_or("");
    eprintln!("[DEBUG] Fetch - Original remote URL: {}", remote_url);

    // Résoudre l'URL (remplacer l'alias SSH par le vrai hostname)
    let resolved_url = resolve_remote_url(remote_url);
    eprintln!("[DEBUG] Fetch - Resolved remote URL: {}", resolved_url);

    // Créer un remote temporaire avec l'URL résolue
    let temp_remote_name = format!("__temp_fetch_{}", std::process::id());

    // Supprimer le remote temporaire s'il existe déjà
    let _ = repo.remote_delete(&temp_remote_name);

    // Créer le remote avec l'URL résolue
    let mut resolved_remote = repo.remote(&temp_remote_name, &resolved_url)?;

    // Options de fetch avec callbacks SSH améliorés
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(build_remote_callbacks());

    // Fetch toutes les branches
    let result = resolved_remote.fetch(
        &["refs/heads/*:refs/remotes/origin/*"],
        Some(&mut fetch_options),
        None,
    );

    // Nettoyer le remote temporaire
    drop(resolved_remote);
    let _ = repo.remote_delete(&temp_remote_name);

    result?;
    Ok(())
}

/// Vérifie si le repository a un remote configuré.
pub fn has_remote(repo: &Repository) -> Result<bool> {
    let remotes = repo.remotes()?;
    Ok(remotes.len() > 0)
}

/// Récupère le nom du remote par défaut pour la branche courante.
pub fn get_default_remote(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("HEAD détachée"))?;

    // Essayer de récupérer le remote configuré pour la branche
    let remote_name = repo
        .branch_upstream_name(&format!("refs/heads/{}", branch_name))
        .ok()
        .and_then(|name| name.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "origin".to_string());

    // Extraire juste le nom du remote
    let remote_name = remote_name
        .split('/')
        .next()
        .unwrap_or("origin")
        .to_string();

    Ok(remote_name)
}
