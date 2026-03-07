use std::io::Write;
use std::path::Path;

use git2::Repository;

use crate::error::{GitSvError, Result};
use crate::git::conflict::{
    parse_conflict_file, ConflictFile, ConflictResolution, ConflictType, MergeFile,
};

/// Resout un fichier en appliquant les resolutions choisies.
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
            let section = file
                .conflicts
                .get(conflict_idx)
                .ok_or_else(|| GitSvError::Other("Section de conflit non trouvee".into()))?;

            for line in lines.by_ref() {
                if line.starts_with(">>>>>>>") {
                    break;
                }
            }

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
            resolved_content.push_str(line);
            resolved_content.push('\n');
        }
    }

    let mut file_handle = std::fs::File::create(&file.path).map_err(|e| {
        GitSvError::Other(format!(
            "Impossible d'ecrire le fichier '{}': {}",
            file.path, e
        ))
    })?;
    file_handle
        .write_all(resolved_content.as_bytes())
        .map_err(|e| {
            GitSvError::Other(format!(
                "Erreur lors de l'ecriture du fichier '{}': {}",
                file.path, e
            ))
        })?;

    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

    index.remove_path(Path::new(&file.path)).ok();
    index.add_path(Path::new(&file.path)).map_err(|e| {
        GitSvError::Other(format!("Impossible d'ajouter le fichier a l'index: {}", e))
    })?;
    index
        .write()
        .map_err(|e| GitSvError::Other(format!("Impossible d'ecrire l'index: {}", e)))?;

    Ok(())
}

/// Resout tous les conflits d'un fichier avec la meme strategie.
pub fn resolve_file_with_strategy(
    repo: &Repository,
    path: &str,
    strategy: ConflictResolution,
) -> Result<()> {
    let mut sections = parse_conflict_file(path)?;

    for section in &mut sections {
        section.resolution = Some(strategy);
    }

    let file = ConflictFile {
        path: path.to_string(),
        conflicts: sections,
        is_resolved: true,
        conflict_type: ConflictType::BothModified,
    };

    resolve_file(repo, &file)
}

/// Resout un fichier de conflit special.
pub fn resolve_special_file(
    repo: &Repository,
    file: &MergeFile,
    resolution: ConflictResolution,
) -> Result<bool> {
    let should_delete = match file.conflict_type {
        Some(ConflictType::DeletedByUs) => matches!(resolution, ConflictResolution::Ours),
        Some(ConflictType::DeletedByThem) => matches!(resolution, ConflictResolution::Theirs),
        Some(ConflictType::BothAdded) => false,
        _ => {
            return Err(GitSvError::Other(
                "Type de conflit non supporte pour resolve_special_file".into(),
            ));
        }
    };

    let path = Path::new(&file.path);

    if should_delete {
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| {
                GitSvError::Other(format!(
                    "Impossible de supprimer le fichier '{}': {}",
                    file.path, e
                ))
            })?;
        }

        let mut index = repo
            .index()
            .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

        index.remove_path(path).map_err(|e| {
            GitSvError::Other(format!(
                "Impossible de retirer le fichier de l'index: {}",
                e
            ))
        })?;
        index
            .write()
            .map_err(|e| GitSvError::Other(format!("Impossible d'ecrire l'index: {}", e)))?;

        Ok(true)
    } else {
        let content = match file.conflict_type {
            Some(ConflictType::DeletedByUs) => file
                .conflicts
                .first()
                .map(|s| s.theirs.join("\n"))
                .unwrap_or_default(),
            Some(ConflictType::DeletedByThem) => file
                .conflicts
                .first()
                .map(|s| s.ours.join("\n"))
                .unwrap_or_default(),
            Some(ConflictType::BothAdded) => {
                let section = file.conflicts.first();
                match resolution {
                    ConflictResolution::Ours => {
                        section.map(|s| s.ours.join("\n")).unwrap_or_default()
                    }
                    ConflictResolution::Theirs => {
                        section.map(|s| s.theirs.join("\n")).unwrap_or_default()
                    }
                    ConflictResolution::Both => section
                        .map(|s| {
                            let mut result = s.ours.clone();
                            result.extend(s.theirs.clone());
                            result.join("\n")
                        })
                        .unwrap_or_default(),
                }
            }
            _ => String::new(),
        };

        let mut file_handle = std::fs::File::create(path).map_err(|e| {
            GitSvError::Other(format!(
                "Impossible de creer le fichier '{}': {}",
                file.path, e
            ))
        })?;
        file_handle.write_all(content.as_bytes()).map_err(|e| {
            GitSvError::Other(format!(
                "Erreur lors de l'ecriture du fichier '{}': {}",
                file.path, e
            ))
        })?;

        let mut index = repo
            .index()
            .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

        index.remove_path(path).ok();
        index.add_path(path).map_err(|e| {
            GitSvError::Other(format!("Impossible d'ajouter le fichier a l'index: {}", e))
        })?;
        index
            .write()
            .map_err(|e| GitSvError::Other(format!("Impossible d'ecrire l'index: {}", e)))?;

        Ok(false)
    }
}
