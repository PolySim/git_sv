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
    if let Some(content) = &file.resolved_content {
        return content
            .iter()
            .map(|line| ResolvedLine {
                content: line.clone(),
                source: LineSource::Context,
            })
            .collect();
    }
    let mut result: Vec<ResolvedLine> = Vec::new();

    for section in &file.conflicts {
        for line in &section.context_before {
            result.push(ResolvedLine {
                content: line.clone(),
                source: LineSource::Context,
            });
        }

        // Une résolution de bloc reste valable en mode ligne tant qu'aucune ligne n'est éditée.
        let mode = if mode == ConflictResolutionMode::Line && section.resolution.is_some() {
            ConflictResolutionMode::Block
        } else {
            mode
        };
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
pub fn all_sections_resolved(file: &MergeFile, mode: ConflictResolutionMode) -> bool {
    !file.conflicts.is_empty()
        && file.conflicts.iter().all(|c| {
            c.resolution.is_some()
                || (mode == ConflictResolutionMode::Line
                    && c.line_level_resolution
                        .as_ref()
                        .is_some_and(|lr| lr.touched))
        })
}

/// Applique le contenu resolu d'un fichier sur le disque et met a jour l'index git.
pub fn apply_resolved_content(
    repo: &Repository,
    file: &MergeFile,
    mode: ConflictResolutionMode,
) -> Result<()> {
    if !all_sections_resolved(file, mode) {
        return Err(GitSvError::Other(
            "Résolvez tous les blocs avant d'écrire le fichier".into(),
        ));
    }
    let path = super::worktree_path(repo, &file.path)?;
    let content = generate_resolved_content_with_source(file, mode)
        .into_iter()
        .map(|line| line.content)
        .collect::<Vec<_>>()
        .join("\n");
    // Un échec d'index peut laisser le bon résultat sur disque : autoriser la même sauvegarde.
    if std::fs::read_to_string(&path)? != content {
        let current = super::parse_conflict_file(&path)?;
        if current.len() != file.conflicts.len()
            || current.iter().zip(&file.conflicts).any(|(now, loaded)| {
                now.context_before != loaded.context_before
                    || now.ours != loaded.ours
                    || now.theirs != loaded.theirs
                    || now.context_after != loaded.context_after
            })
        {
            return Err(GitSvError::Other("Le fichier a changé depuis le chargement des conflits. Rechargez-le avant de le résoudre.".into()));
        }
        std::fs::write(&path, &content)?;
    }
    super::stage_resolution(repo, Path::new(&file.path), false)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::conflict::{parse_conflict_file, ConflictType};
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn line_selection_does_not_resolve_a_block_that_still_contains_markers() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "file.txt", "base\n", "initial");
        let path = temp.path().join("file.txt");
        let content = "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> topic\n";
        std::fs::write(&path, content).unwrap();
        let mut sections = parse_conflict_file(path.to_str().unwrap()).unwrap();
        sections[0].line_level_resolution.as_mut().unwrap().touched = true;
        let file = MergeFile {
            resolved_content: None,
            path: "file.txt".into(),
            has_conflicts: true,
            conflicts: sections,
            is_resolved: false,
            conflict_type: Some(ConflictType::BothModified),
        };
        assert!(apply_resolved_content(&repo, &file, ConflictResolutionMode::Block).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), content);
    }

    #[test]
    fn switching_to_line_mode_keeps_the_chosen_block_resolution() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "file.txt", "base\n", "initial");
        let path = temp.path().join("file.txt");
        std::fs::write(
            &path,
            "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> topic\n",
        )
        .unwrap();
        let mut sections = parse_conflict_file(path.to_str().unwrap()).unwrap();
        sections[0].resolution = Some(ConflictResolution::Theirs);
        let file = MergeFile {
            resolved_content: None,
            path: "file.txt".into(),
            has_conflicts: true,
            conflicts: sections,
            is_resolved: false,
            conflict_type: Some(ConflictType::BothModified),
        };
        apply_resolved_content(&repo, &file, ConflictResolutionMode::Line).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "theirs\n");
    }
    #[test]
    fn resolution_refuses_to_overwrite_a_file_changed_after_loading() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "file.txt", "base\n", "initial");
        let path = temp.path().join("file.txt");
        let content = "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> topic\n";
        std::fs::write(&path, content).unwrap();
        let mut sections = parse_conflict_file(&path).unwrap();
        sections[0].resolution = Some(ConflictResolution::Theirs);
        let file = MergeFile {
            resolved_content: None,
            path: "file.txt".into(),
            has_conflicts: true,
            conflicts: sections,
            is_resolved: false,
            conflict_type: Some(ConflictType::BothModified),
        };
        std::fs::write(&path, "résolution externe\n").unwrap();
        assert!(apply_resolved_content(&repo, &file, ConflictResolutionMode::Block).is_err());
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            "résolution externe\n"
        );
    }
    #[test]
    fn index_lock_does_not_leak_a_resolution_into_the_cached_index() {
        let (temp, repo) = create_test_repo();
        commit_file(&repo, "file.txt", "base\n", "initial");
        let path = temp.path().join("file.txt");
        std::fs::write(
            &path,
            "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> topic\n",
        )
        .unwrap();
        let mut sections = parse_conflict_file(&path).unwrap();
        sections[0].resolution = Some(ConflictResolution::Theirs);
        let file = MergeFile {
            resolved_content: None,
            path: "file.txt".into(),
            has_conflicts: true,
            conflicts: sections,
            is_resolved: false,
            conflict_type: Some(ConflictType::BothModified),
        };
        let before = repo
            .index()
            .unwrap()
            .get_path(Path::new("file.txt"), 0)
            .unwrap()
            .id;
        let lock = repo.path().join("index.lock");
        std::fs::write(&lock, "locked").unwrap();
        assert!(apply_resolved_content(&repo, &file, ConflictResolutionMode::Block).is_err());
        assert_eq!(
            repo.index()
                .unwrap()
                .get_path(Path::new("file.txt"), 0)
                .unwrap()
                .id,
            before
        );
        std::fs::remove_file(lock).unwrap();
        apply_resolved_content(&repo, &file, ConflictResolutionMode::Block).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "theirs\n");
    }
}
