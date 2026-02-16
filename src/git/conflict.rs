use git2::Repository;
use std::io::Write;

use crate::error::{GitSvError, Result};

/// Un fichier en conflit.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictFile {
    pub path: String,
    pub conflicts: Vec<ConflictSection>,
    pub is_resolved: bool,
}

/// Une section de conflit dans un fichier.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictSection {
    /// Lignes de contexte avant le conflit.
    pub context_before: Vec<String>,
    /// Version "ours" (HEAD / branche courante).
    pub ours: Vec<String>,
    /// Version "theirs" (branche mergée).
    pub theirs: Vec<String>,
    /// Lignes de contexte après le conflit.
    pub context_after: Vec<String>,
    /// Résolution choisie par l'utilisateur.
    pub resolution: Option<ConflictResolution>,
}

/// Résolution possible pour une section de conflit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictResolution {
    Ours,
    Theirs,
    Both,
}

/// Résultat d'une opération de merge.
#[derive(Debug)]
pub enum MergeResult {
    Success,
    FastForward,
    UpToDate,
    Conflicts(Vec<ConflictFile>),
}

/// Parser les marqueurs de conflit dans un fichier.
pub fn parse_conflict_file(path: &str) -> Result<Vec<ConflictSection>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        GitSvError::Other(format!("Impossible de lire le fichier '{}': {}", path, e))
    })?;

    let mut sections = Vec::new();
    let mut lines = content.lines().peekable();
    let mut context_before: Vec<String> = Vec::new();

    while let Some(line) = lines.next() {
        if line.starts_with("<<<<<<<") {
            // Début d'une section de conflit
            let mut ours: Vec<String> = Vec::new();
            let mut theirs: Vec<String> = Vec::new();
            let mut in_ours = true;

            // Parser jusqu'au séparateur ou fin
            while let Some(line) = lines.next() {
                if line == "=======" {
                    in_ours = false;
                } else if line.starts_with(">>>>>>>") {
                    // Fin de la section de conflit
                    let context_after = collect_context_after(&mut lines, 3);

                    sections.push(ConflictSection {
                        context_before: context_before.clone(),
                        ours,
                        theirs,
                        context_after,
                        resolution: None,
                    });

                    context_before = Vec::new();
                    break;
                } else if in_ours {
                    ours.push(line.to_string());
                } else {
                    theirs.push(line.to_string());
                }
            }
        } else {
            // Garder les lignes de contexte (max 3 avant le prochain conflit)
            context_before.push(line.to_string());
            if context_before.len() > 3 {
                context_before.remove(0);
            }
        }
    }

    Ok(sections)
}

/// Collecte les lignes de contexte après un conflit.
fn collect_context_after(
    lines: &mut std::iter::Peekable<std::str::Lines>,
    max: usize,
) -> Vec<String> {
    let mut context = Vec::new();
    for _ in 0..max {
        if let Some(line) = lines.peek() {
            if !line.starts_with("<<<<<<<") {
                context.push(line.to_string());
                lines.next();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    context
}

/// Liste tous les fichiers en conflit dans le repository.
pub fn list_conflict_files(repo: &Repository) -> Result<Vec<ConflictFile>> {
    let index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

    let conflicts = index
        .conflicts()
        .map_err(|e| GitSvError::Other(format!("Impossible de lister les conflits: {}", e)))?;

    let mut files = Vec::new();

    for conflict in conflicts {
        let conflict = conflict
            .map_err(|e| GitSvError::Other(format!("Erreur lors du parsing du conflit: {}", e)))?;

        if let Some(ours) = conflict.our {
            let path = std::str::from_utf8(&ours.path)
                .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                .to_string();

            // Parser les sections de conflit du fichier
            let sections = parse_conflict_file(&path)?;
            let is_resolved = sections.iter().all(|s| s.resolution.is_some());

            files.push(ConflictFile {
                path,
                conflicts: sections,
                is_resolved,
            });
        }
    }

    Ok(files)
}

/// Vérifie si le repository a des conflits non résolus.
pub fn has_conflicts(repo: &Repository) -> Result<bool> {
    let index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

    Ok(index.has_conflicts())
}

/// Résout un fichier en appliquant les résolutions choisies.
pub fn resolve_file(repo: &Repository, file: &ConflictFile) -> Result<()> {
    let content = std::fs::read_to_string(&file.path).map_err(|e| {
        GitSvError::Other(format!(
            "Impossible de lire le fichier '{}': {}",
            file.path, e
        ))
    })?;

    let mut resolved_content = String::new();
    let mut conflict_idx = 0;
    let mut lines = content.lines();

    while let Some(line) = lines.next() {
        if line.starts_with("<<<<<<<") {
            // C'est une section de conflit
            let section = file
                .conflicts
                .get(conflict_idx)
                .ok_or_else(|| GitSvError::Other("Section de conflit non trouvée".into()))?;

            // Sauter toute la section de conflit
            while let Some(line) = lines.next() {
                if line.starts_with(">>>>>>>") {
                    break;
                }
            }

            // Appliquer la résolution
            match section.resolution {
                Some(ConflictResolution::Ours) => {
                    for l in &section.ours {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                }
                Some(ConflictResolution::Theirs) => {
                    for l in &section.theirs {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                }
                Some(ConflictResolution::Both) => {
                    for l in &section.ours {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                    for l in &section.theirs {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                }
                None => {
                    // Pas de résolution, garder le conflit tel quel
                    resolved_content.push_str(line);
                    resolved_content.push('\n');
                    for l in &section.ours {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                    resolved_content.push_str("=======\n");
                    for l in &section.theirs {
                        resolved_content.push_str(l);
                        resolved_content.push('\n');
                    }
                    resolved_content.push_str(&format!(">>>>>>> {}\n", "HEAD"));
                }
            }

            conflict_idx += 1;
        } else {
            // Ligne normale
            resolved_content.push_str(line);
            resolved_content.push('\n');
        }
    }

    // Écrire le fichier résolu
    let mut file_handle = std::fs::File::create(&file.path).map_err(|e| {
        GitSvError::Other(format!(
            "Impossible d'écrire le fichier '{}': {}",
            file.path, e
        ))
    })?;
    file_handle
        .write_all(resolved_content.as_bytes())
        .map_err(|e| {
            GitSvError::Other(format!(
                "Erreur lors de l'écriture du fichier '{}': {}",
                file.path, e
            ))
        })?;

    // Ajouter le fichier à l'index (git add)
    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;
    index
        .add_path(std::path::Path::new(&file.path))
        .map_err(|e| {
            GitSvError::Other(format!("Impossible d'ajouter le fichier à l'index: {}", e))
        })?;
    index
        .write()
        .map_err(|e| GitSvError::Other(format!("Impossible d'écrire l'index: {}", e)))?;

    Ok(())
}

/// Annule le merge en cours (cleanup_state).
pub fn abort_merge(repo: &Repository) -> Result<()> {
    repo.cleanup_state()
        .map_err(|e| GitSvError::Other(format!("Impossible d'annuler le merge: {}", e)))?;

    // Réinitialiser les fichiers modifiés
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force();
    repo.checkout_head(Some(&mut checkout_builder))
        .map_err(|e| GitSvError::Other(format!("Erreur lors du checkout: {}", e)))?;

    Ok(())
}

/// Finalise le merge en créant le commit de merge.
pub fn finalize_merge(repo: &Repository, message: &str) -> Result<()> {
    // Vérifier qu'il n'y a plus de conflits
    if has_conflicts(repo)? {
        return Err(GitSvError::Other(
            "Des conflits non résolus restent. Résolvez-les avant de finaliser.".into(),
        ));
    }

    let signature = repo
        .signature()
        .map_err(|e| GitSvError::Other(format!("Impossible d'obtenir la signature: {}", e)))?;

    let head = repo
        .head()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à HEAD: {}", e)))?;
    let head_commit = head
        .peel_to_commit()
        .map_err(|e| GitSvError::Other(format!("Impossible de résoudre HEAD: {}", e)))?;

    // Obtenir l'index
    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'accéder à l'index: {}", e)))?;

    // Écrire l'arbre
    let tree_id = index
        .write_tree()
        .map_err(|e| GitSvError::Other(format!("Impossible d'écrire l'arbre: {}", e)))?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|e| GitSvError::Other(format!("Impossible de trouver l'arbre: {}", e)))?;

    // Trouver les parents (HEAD + branche mergée si c'est un merge)
    // Vérifier s'il y a un MERGE_HEAD
    let merge_head_oid = std::fs::read_to_string(repo.path().join("MERGE_HEAD"))
        .ok()
        .and_then(|content| git2::Oid::from_str(content.trim()).ok());

    let merge_commit = merge_head_oid.and_then(|oid| repo.find_commit(oid).ok());

    // Créer le commit
    let _commit_oid = if let Some(ref merge_commit) = merge_commit {
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit, merge_commit],
        )
    } else {
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit],
        )
    }
    .map_err(|e| GitSvError::Other(format!("Impossible de créer le commit: {}", e)))?;

    // Nettoyer l'état de merge
    repo.cleanup_state()
        .map_err(|e| GitSvError::Other(format!("Impossible de nettoyer l'état de merge: {}", e)))?;

    Ok(())
}

/// Résout tous les conflits d'un fichier avec la même stratégie.
pub fn resolve_file_with_strategy(
    repo: &Repository,
    path: &str,
    strategy: ConflictResolution,
) -> Result<()> {
    let mut sections = parse_conflict_file(path)?;

    // Appliquer la stratégie à toutes les sections
    for section in &mut sections {
        section.resolution = Some(strategy);
    }

    let file = ConflictFile {
        path: path.to_string(),
        conflicts: sections,
        is_resolved: true,
    };

    resolve_file(repo, &file)
}

/// Compte le nombre de fichiers en conflit non résolus.
pub fn count_unresolved_files(files: &[ConflictFile]) -> usize {
    files.iter().filter(|f| !f.is_resolved).count()
}

/// Compte le nombre total de sections de conflit non résolues.
pub fn count_unresolved_sections(files: &[ConflictFile]) -> usize {
    files
        .iter()
        .flat_map(|f| &f.conflicts)
        .filter(|s| s.resolution.is_none())
        .count()
}

/// Met à jour le statut resolved d'un fichier basé sur ses sections.
pub fn update_file_resolved_status(file: &mut ConflictFile) {
    file.is_resolved = file.conflicts.iter().all(|s| s.resolution.is_some());
}
