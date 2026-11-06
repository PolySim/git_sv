use git2::Repository;

use crate::error::{GitSvError, Result};
use crate::git::conflict::{ConflictFile, ConflictSection, ConflictType, LineLevelResolution};

/// Parser les marqueurs de conflit dans un fichier.
pub fn parse_conflict_file(path: &str) -> Result<Vec<ConflictSection>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        GitSvError::Other(format!("Impossible de lire le fichier '{}': {}", path, e))
    })?;

    let mut sections = Vec::new();
    let mut lines = content.split('\n').peekable();
    let mut context_before = Vec::new();

    while let Some(line) = lines.next() {
        if !line.starts_with("<<<<<<<") {
            context_before.push(line.to_string());
            continue;
        }

        let mut ours = Vec::new();
        let mut theirs = Vec::new();
        let mut side = 0;
        let mut closed = false;
        for line in lines.by_ref() {
            if line.trim_end_matches('\r') == "=======" {
                side = 2;
            } else if line.starts_with("|||||||") {
                // Les styles diff3 et zdiff3 incluent l'ancêtre, jamais une résolution.
                side = 1;
            } else if line.starts_with(">>>>>>>") {
                closed = side == 2;
                break;
            } else {
                match side {
                    0 => ours.push(line.to_string()),
                    2 => theirs.push(line.to_string()),
                    _ => {}
                }
            }
        }
        if !closed {
            return Err(GitSvError::Other("Marqueurs de conflit incomplets".into()));
        }
        let line_resolution = LineLevelResolution::new(ours.len(), theirs.len());
        sections.push(ConflictSection {
            context_before: std::mem::take(&mut context_before),
            ours,
            theirs,
            context_after: Vec::new(),
            resolution: None,
            line_level_resolution: Some(line_resolution),
        });
    }
    if let Some(last) = sections.last_mut() {
        last.context_after = context_before;
    }
    Ok(sections)
}

/// Liste tous les fichiers en conflit dans le repository.
pub fn list_conflict_files(repo: &Repository) -> Result<Vec<ConflictFile>> {
    let mut index = repo
        .index()
        .map_err(|e| GitSvError::Other(format!("Impossible d'acceder a l'index: {}", e)))?;

    index.read(true)?;
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

        let full_path = super::worktree_path(repo, &path)?;
        let sections = match conflict_type {
            ConflictType::BothModified | ConflictType::BothAdded => {
                parse_conflict_file(full_path.to_string_lossy().as_ref())?
            }
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
                    line_level_resolution: Some(LineLevelResolution::new(0, theirs_content.len())),
                }]
            }
            ConflictType::DeletedByThem => {
                let ours_content = read_blob_content(
                    repo,
                    conflict.our.as_ref().ok_or_else(|| {
                        GitSvError::Other("Version locale du conflit introuvable".into())
                    })?,
                )?;
                vec![ConflictSection {
                    context_before: vec![],
                    ours: ours_content.clone(),
                    theirs: vec![],
                    context_after: vec![],
                    resolution: None,
                    line_level_resolution: Some(LineLevelResolution::new(ours_content.len(), 0)),
                }]
            }
        };

        let is_resolved = !sections.is_empty() && sections.iter().all(|s| s.resolution.is_some());

        files.push(ConflictFile {
            path,
            conflicts: sections,
            is_resolved,
            conflict_type,
        });
    }

    Ok(files)
}

fn read_blob_content(repo: &Repository, entry: &git2::IndexEntry) -> Result<Vec<String>> {
    let blob = repo
        .find_blob(entry.id)
        .map_err(|e| GitSvError::Other(format!("Impossible de trouver le blob: {}", e)))?;
    let content = std::str::from_utf8(blob.content())
        .map_err(|_| GitSvError::Other("Contenu du blob invalide".into()))?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::conflict::{
        apply_resolved_content, ConflictResolution, ConflictResolutionMode, MergeFile,
    };
    use crate::git::tests::test_utils::{commit_file, create_test_repo};

    #[test]
    fn resolution_preserves_all_context_and_line_endings_from_another_directory() {
        for newline in ["\n", "\r\n"] {
            for trailing_newline in [false, true] {
                let (directory, repo) = create_test_repo();
                let before = (0..10)
                    .map(|i| format!("before{i}"))
                    .collect::<Vec<_>>()
                    .join(newline);
                let after = (0..10)
                    .map(|i| format!("after{i}"))
                    .collect::<Vec<_>>()
                    .join(newline);
                let suffix = if trailing_newline { newline } else { "" };
                let content = format!("{before}{newline}<<<<<<< HEAD{newline}ours{newline}||||||| base{newline}ancestor{newline}======={newline}theirs{newline}>>>>>>> other{newline}{after}{suffix}");
                commit_file(&repo, "nested/file.txt", "initial", "initial");
                let path = directory.path().join("nested/file.txt");
                std::fs::write(&path, &content).unwrap();
                let mut sections = parse_conflict_file(path.to_str().unwrap()).unwrap();
                assert_eq!(sections.len(), 1);
                sections[0].resolution = Some(ConflictResolution::Ours);
                let file = MergeFile {
                    path: "nested/file.txt".into(),
                    has_conflicts: true,
                    conflicts: sections,
                    is_resolved: true,
                    conflict_type: Some(ConflictType::BothModified),
                };
                apply_resolved_content(&repo, &file, ConflictResolutionMode::Block).unwrap();
                let expected = format!("{before}{newline}ours{newline}{after}{suffix}");
                assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
                let entry = repo
                    .index()
                    .unwrap()
                    .get_path(std::path::Path::new("nested/file.txt"), 0)
                    .unwrap();
                assert_eq!(
                    repo.find_blob(entry.id).unwrap().content(),
                    expected.as_bytes()
                );
            }
        }
    }

    #[test]
    fn parser_rejects_unclosed_conflicts() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "prefix\n<<<<<<< HEAD\nours\n=======\ntheirs\n").unwrap();
        assert!(parse_conflict_file(file.path().to_str().unwrap()).is_err());
    }
}
