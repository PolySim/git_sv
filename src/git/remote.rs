use crate::error::Result;
use git2::{FetchOptions, PushOptions, RemoteCallbacks, Repository};

/// Push la branche courante vers le remote.
pub fn push_current_branch(repo: &Repository) -> Result<()> {
    // Récupérer la branche courante
    let head = repo.head()?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("HEAD détachée, impossible de pousser"))?;

    // Récupérer le nom du remote associé à la branche
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

    // Récupérer le remote
    let mut remote = repo.find_remote(&remote_name)?;

    // Configurer les callbacks
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
    });

    // Options de push
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    // Pousser la branche courante
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[&refspec], Some(&mut push_options))?;

    Ok(())
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
    let mut remote = repo.find_remote(remote_name)?;

    // Configurer les callbacks
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
    });

    // Options de fetch
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    // Fetch toutes les branches
    remote.fetch(
        &["refs/heads/*:refs/remotes/origin/*"],
        Some(&mut fetch_options),
        None,
    )?;

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
