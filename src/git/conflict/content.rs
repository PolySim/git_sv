use std::path::Path;

use git2::Repository;

use crate::error::{GitSvError, Result};
use crate::git::conflict::{
    ConflictResolution, ConflictResolutionMode, LineSource, MergeFile, ResolvedLine,
};

/// Genere le contenu resolu avec provenance de chaque ligne.
pub fn generate_resolved_content_with_source(
    file: &MergeFile,
    mode: ConflictResolutionMode,
) -> Vec<ResolvedLine> {
    let mut result: Vec<ResolvedLine> = Vec::new();

    for section in &file.conflicts {
        for line in &section.context_before {
            result.push(ResolvedLine {
                content: line.clone(),
                source: LineSource::Context,
            });
        }

        match mode {
            ConflictResolutionMode::File | ConflictResolutionMode::Block => {
                if let Some(resolution) = &section.resolution {
                    match resolution {
                        ConflictResolution::Ours => {
                            for line in &section.ours {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Ours,
                                });
                            }
                        }
                        ConflictResolution::Theirs => {
                            for line in &section.theirs {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Theirs,
                                });
                            }
                        }
                        ConflictResolution::Both => {
                            for line in &section.ours {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Ours,
                                });
                            }
                            for line in &section.theirs {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Theirs,
                                });
                            }
                        }
                    }
                } else {
                    result.push(ResolvedLine {
                        content: "<<<<<<< HEAD".into(),
                        source: LineSource::ConflictMarker,
                    });
                    for line in &section.ours {
                        result.push(ResolvedLine {
                            content: line.clone(),
                            source: LineSource::Ours,
                        });
                    }
                    result.push(ResolvedLine {
                        content: "=======".into(),
                        source: LineSource::ConflictMarker,
                    });
                    for line in &section.theirs {
                        result.push(ResolvedLine {
                            content: line.clone(),
                            source: LineSource::Theirs,
                        });
                    }
                    result.push(ResolvedLine {
                        content: ">>>>>>>".into(),
                        source: LineSource::ConflictMarker,
                    });
                }
            }
            ConflictResolutionMode::Line => {
                if let Some(ref lr) = section.line_level_resolution {
                    for (i, line) in section.ours.iter().enumerate() {
                        if lr.ours_lines_included.get(i) == Some(&true) {
                            result.push(ResolvedLine {
                                content: line.clone(),
                                source: LineSource::Ours,
                            });
                        }
                    }
                    for (i, line) in section.theirs.iter().enumerate() {
                        if lr.theirs_lines_included.get(i) == Some(&true) {
                            result.push(ResolvedLine {
                                content: line.clone(),
                                source: LineSource::Theirs,
                            });
                        }
                    }
                } else {
                    for line in &section.ours {
                        result.push(ResolvedLine {
                            content: line.clone(),
                            source: LineSource::Ours,
                        });
                    }
                }
            }
        }

        for line in &section.context_after {
            result.push(ResolvedLine {
                content: line.clone(),
                source: LineSource::Context,
            });
        }
    }

    result
}

/// Verifie si toutes les sections d'un fichier sont resolues.
pub fn all_sections_resolved(file: &MergeFile) -> bool {
    file.conflicts.iter().all(|c| {
        c.resolution.is_some()
            || c.line_level_resolution
                .as_ref()
                .is_some_and(|lr| lr.touched)
    })
}

/// Applique le contenu resolu d'un fichier sur le disque et met a jour l'index git.
pub fn apply_resolved_content(
    repo: &Repository,
    file: &MergeFile,
    mode: ConflictResolutionMode,
) -> Result<()> {
    let resolved_lines = generate_resolved_content_with_source(file, mode);
    let content: String = resolved_lines
        .into_iter()
        .map(|line| line.content)
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(&file.path, &content).map_err(|e| {
        GitSvError::Other(format!(
            "Impossible d'ecrire le fichier '{}': {}",
            file.path, e
        ))
    })?;

    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

    index.remove_path(Path::new(&file.path)).map_err(|e| {
        GitSvError::Other(format!(
            "Impossible de supprimer le conflit de l'index: {}",
            e
        ))
    })?;

    index.add_path(Path::new(&file.path)).map_err(|e| {
        GitSvError::Other(format!("Impossible d'ajouter le fichier a l'index: {}", e))
    })?;

    index
        .write()
        .map_err(|e| GitSvError::Other(format!("Impossible d'ecrire l'index: {}", e)))?;

    Ok(())
}
