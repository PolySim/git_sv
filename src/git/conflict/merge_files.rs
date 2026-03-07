use git2::Repository;

use crate::error::{GitSvError, Result};
use crate::git::conflict::{
    parse_conflict_file, ConflictFile, ConflictSection, ConflictType, LineLevelResolution,
    MergeFile,
};

/// Compte le nombre de fichiers en conflit non resolus.
pub fn count_unresolved_files(files: &[ConflictFile]) -> usize {
    files.iter().filter(|f| !f.is_resolved).count()
}

/// Compte le nombre total de sections de conflit non resolues.
pub fn count_unresolved_sections(files: &[ConflictFile]) -> usize {
    files
        .iter()
        .flat_map(|f| &f.conflicts)
        .filter(|s| s.resolution.is_none())
        .count()
}

/// Met a jour le statut resolved d'un fichier base sur ses sections.
pub fn update_file_resolved_status(file: &mut ConflictFile) {
    file.is_resolved = file.conflicts.iter().all(|s| s.resolution.is_some());
}

/// Liste tous les fichiers du merge (en conflit ou non).
pub fn list_all_merge_files(repo: &Repository) -> Result<Vec<MergeFile>> {
    let index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

    let mut all_files: Vec<MergeFile> = Vec::new();
    let mut conflict_map: std::collections::HashMap<String, ConflictType> =
        std::collections::HashMap::new();

    if let Ok(conflicts) = index.conflicts() {
        for conflict in conflicts.filter_map(|c| c.ok()) {
            let (path, conflict_type) = match (&conflict.our, &conflict.their, &conflict.ancestor) {
                (Some(ours), Some(_theirs), Some(_ancestor)) => {
                    if let Ok(p) = std::str::from_utf8(&ours.path) {
                        (p.to_string(), ConflictType::BothModified)
                    } else {
                        continue;
                    }
                }
                (Some(ours), None, _) => {
                    if let Ok(p) = std::str::from_utf8(&ours.path) {
                        (p.to_string(), ConflictType::DeletedByThem)
                    } else {
                        continue;
                    }
                }
                (None, Some(theirs), _) => {
                    if let Ok(p) = std::str::from_utf8(&theirs.path) {
                        (p.to_string(), ConflictType::DeletedByUs)
                    } else {
                        continue;
                    }
                }
                (Some(ours), Some(_theirs), None) => {
                    if let Ok(p) = std::str::from_utf8(&ours.path) {
                        (p.to_string(), ConflictType::BothAdded)
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };
            conflict_map.insert(path, conflict_type);
        }
    }

    for i in 0..index.len() {
        if let Some(entry) = index.get(i) {
            let path_bytes = entry.path;
            if let Ok(path) = std::str::from_utf8(&path_bytes) {
                if let Some(&conflict_type) = conflict_map.get(path) {
                    let sections = match conflict_type {
                        ConflictType::BothModified | ConflictType::BothAdded => {
                            parse_conflict_file(path).unwrap_or_default()
                        }
                        ConflictType::DeletedByUs => {
                            let theirs_content = if let Ok(conflicts) = index.conflicts() {
                                conflicts
                                    .filter_map(|c| c.ok())
                                    .find(|c| {
                                        c.their.as_ref().is_some_and(|t| {
                                            std::str::from_utf8(&t.path).ok() == Some(path)
                                        })
                                    })
                                    .and_then(|c| c.their)
                                    .and_then(|entry| read_blob_content(repo, &entry).ok())
                                    .unwrap_or_default()
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
                                line_level_resolution: Some(LineLevelResolution::new(
                                    0,
                                    theirs_content.len(),
                                )),
                            }]
                        }
                        ConflictType::DeletedByThem => {
                            let ours_content = read_file_lines(path).unwrap_or_default();
                            vec![ConflictSection {
                                context_before: vec![],
                                ours: ours_content.clone(),
                                theirs: vec![],
                                context_after: vec![],
                                resolution: None,
                                line_resolutions: vec![],
                                line_level_resolution: Some(LineLevelResolution::new(
                                    ours_content.len(),
                                    0,
                                )),
                            }]
                        }
                    };

                    let is_resolved = sections.iter().all(|s| s.resolution.is_some());

                    all_files.push(MergeFile {
                        path: path.to_string(),
                        has_conflicts: true,
                        conflicts: sections,
                        is_resolved,
                        conflict_type: Some(conflict_type),
                    });
                } else {
                    all_files.push(MergeFile {
                        path: path.to_string(),
                        has_conflicts: false,
                        conflicts: Vec::new(),
                        is_resolved: true,
                        conflict_type: None,
                    });
                }
            }
        }
    }

    for (path, conflict_type) in &conflict_map {
        if !all_files.iter().any(|f| &f.path == path) {
            let sections = match conflict_type {
                ConflictType::DeletedByUs => {
                    let theirs_content = if let Ok(conflicts) = index.conflicts() {
                        conflicts
                            .filter_map(|c| c.ok())
                            .find(|c| {
                                c.their.as_ref().is_some_and(|t| {
                                    std::str::from_utf8(&t.path).ok().map(|s| s.to_string())
                                        == Some(path.clone())
                                })
                            })
                            .and_then(|c| c.their)
                            .and_then(|entry| read_blob_content(repo, &entry).ok())
                            .unwrap_or_default()
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
                        line_level_resolution: Some(LineLevelResolution::new(
                            0,
                            theirs_content.len(),
                        )),
                    }]
                }
                _ => vec![],
            };

            let is_resolved = sections.iter().all(|s| s.resolution.is_some());

            all_files.push(MergeFile {
                path: path.clone(),
                has_conflicts: !sections.is_empty(),
                conflicts: sections,
                is_resolved,
                conflict_type: Some(*conflict_type),
            });
        }
    }

    if let Ok(status) = repo.statuses(None) {
        for entry in status.iter() {
            if let Some(path) = entry.path() {
                if !all_files.iter().any(|f| f.path == path) {
                    let status = entry.status();
                    if status.is_wt_new() || status.is_index_new() {
                        all_files.push(MergeFile {
                            path: path.to_string(),
                            has_conflicts: false,
                            conflicts: Vec::new(),
                            is_resolved: true,
                            conflict_type: None,
                        });
                    }
                }
            }
        }
    }

    Ok(all_files)
}

/// Compte le nombre de fichiers en conflit non resolus dans MergeFile.
pub fn count_unresolved_merge_files(files: &[MergeFile]) -> usize {
    files
        .iter()
        .filter(|f| f.has_conflicts && !f.is_resolved)
        .count()
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
