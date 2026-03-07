use std::collections::VecDeque;

use git2::Repository;

use crate::error::{GitSvError, Result};
use crate::git::conflict::{ConflictFile, ConflictSection, ConflictType, LineLevelResolution};

/// Parser les marqueurs de conflit dans un fichier.
pub fn parse_conflict_file(path: &str) -> Result<Vec<ConflictSection>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        GitSvError::Other(format!("Impossible de lire le fichier '{}': {}", path, e))
    })?;

    let mut sections = Vec::new();
    let mut lines = content.lines().peekable();
    let mut context_before: VecDeque<String> = VecDeque::new();

    while let Some(line) = lines.next() {
        if line.starts_with("<<<<<<<") {
            let mut ours: Vec<String> = Vec::new();
            let mut theirs: Vec<String> = Vec::new();
            let mut in_ours = true;

            for line in lines.by_ref() {
                if line == "=======" {
                    in_ours = false;
                } else if line.starts_with(">>>>>>>") {
                    let context_after = collect_context_after(&mut lines, 3);
                    let line_resolution = LineLevelResolution::new(ours.len(), theirs.len());

                    sections.push(ConflictSection {
                        context_before: context_before.iter().cloned().collect(),
                        ours,
                        theirs,
                        context_after,
                        resolution: None,
                        line_resolutions: Vec::new(),
                        line_level_resolution: Some(line_resolution),
                    });

                    context_before.clear();
                    break;
                } else if in_ours {
                    ours.push(line.to_string());
                } else {
                    theirs.push(line.to_string());
                }
            }
        } else {
            context_before.push_back(line.to_string());
            if context_before.len() > 3 {
                context_before.pop_front();
            }
        }
    }

    Ok(sections)
}

/// Liste tous les fichiers en conflit dans le repository.
pub fn list_conflict_files(repo: &Repository) -> Result<Vec<ConflictFile>> {
    let index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

    let conflicts = index
        .conflicts()
        .map_err(|e| GitSvError::Other(format!("Impossible de lister les conflits: {}", e)))?;

    let mut files = Vec::new();

    for conflict in conflicts {
        let conflict = conflict
            .map_err(|e| GitSvError::Other(format!("Erreur lors du parsing du conflit: {}", e)))?;

        let (path, conflict_type) = match (&conflict.our, &conflict.their, &conflict.ancestor) {
            (Some(ours), Some(_theirs), Some(_ancestor)) => {
                let p = std::str::from_utf8(&ours.path)
                    .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                    .to_string();
                (p, ConflictType::BothModified)
            }
            (Some(ours), None, _) => {
                let p = std::str::from_utf8(&ours.path)
                    .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                    .to_string();
                (p, ConflictType::DeletedByThem)
            }
            (None, Some(theirs), _) => {
                let p = std::str::from_utf8(&theirs.path)
                    .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                    .to_string();
                (p, ConflictType::DeletedByUs)
            }
            (Some(ours), Some(_theirs), None) => {
                let p = std::str::from_utf8(&ours.path)
                    .map_err(|_| GitSvError::Other("Chemin de fichier invalide".into()))?
                    .to_string();
                (p, ConflictType::BothAdded)
            }
            _ => continue,
        };

        let sections = match conflict_type {
            ConflictType::BothModified | ConflictType::BothAdded => parse_conflict_file(&path)?,
            ConflictType::DeletedByUs => {
                let theirs_content = if let Some(ref their_entry) = conflict.their {
                    read_blob_content(repo, their_entry)?
                } else {
                    vec![]
                };
                vec![ConflictSection {
                    context_before: vec![],
                    ours: vec![],
                    theirs: theirs_content.clone(),
                    context_after: vec![],
                    resolution: None,
                    line_resolutions: vec![],
                    line_level_resolution: Some(LineLevelResolution::new(0, theirs_content.len())),
                }]
            }
            ConflictType::DeletedByThem => {
                let ours_content = read_file_lines(&path).unwrap_or_default();
                vec![ConflictSection {
                    context_before: vec![],
                    ours: ours_content.clone(),
                    theirs: vec![],
                    context_after: vec![],
                    resolution: None,
                    line_resolutions: vec![],
                    line_level_resolution: Some(LineLevelResolution::new(ours_content.len(), 0)),
                }]
            }
        };

        let is_resolved = sections.iter().all(|s| s.resolution.is_some());

        files.push(ConflictFile {
            path,
            conflicts: sections,
            is_resolved,
            conflict_type,
        });
    }

    Ok(files)
}

/// Collecte les lignes de contexte apres un conflit.
fn collect_context_after(
    lines: &mut std::iter::Peekable<std::str::Lines<'_>>,
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

fn read_blob_content(repo: &Repository, entry: &git2::IndexEntry) -> Result<Vec<String>> {
    let blob = repo
        .find_blob(entry.id)
        .map_err(|e| GitSvError::Other(format!("Impossible de trouver le blob: {}", e)))?;
    let content = std::str::from_utf8(blob.content())
        .map_err(|_| GitSvError::Other("Contenu du blob invalide".into()))?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}

fn read_file_lines(path: &str) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        GitSvError::Other(format!("Impossible de lire le fichier '{}': {}", path, e))
    })?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}
