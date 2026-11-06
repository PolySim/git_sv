use std::path::Path;

use git2::Repository;

use crate::error::{GitSvError, Result};
use crate::git::conflict::{
    parse_conflict_file, ConflictFile, ConflictResolution, ConflictType, MergeFile,
};

/// Resout un fichier en appliquant les resolutions choisies.
pub fn resolve_file(repo: &Repository, file: &ConflictFile) -> Result<()> {
    let merge_file = MergeFile {
        path: file.path.clone(),
        has_conflicts: true,
        conflicts: file.conflicts.clone(),
        is_resolved: file.is_resolved,
        conflict_type: Some(file.conflict_type),
    };
    super::apply_resolved_content(repo, &merge_file, super::ConflictResolutionMode::Block)
}

/// Resout tous les conflits d'un fichier avec la meme strategie.
pub fn resolve_file_with_strategy(
    repo: &Repository,
    path: &str,
    strategy: ConflictResolution,
) -> Result<()> {
    let full_path = super::worktree_path(repo, path)?;
    let mut sections = parse_conflict_file(full_path.to_string_lossy().as_ref())?;

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

    let relative_path = Path::new(&file.path);
    let path = super::worktree_path(repo, &file.path)?;
    let mut index = repo.index()?;
    index.read(true)?;

    if should_delete {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        index.remove_path(relative_path)?;
        index.write()?;
        return Ok(true);
    }

    if file.conflict_type == Some(ConflictType::BothAdded) {
        let mut resolved = file.clone();
        for section in &mut resolved.conflicts {
            section.resolution = Some(resolution);
        }
        super::apply_resolved_content(repo, &resolved, super::ConflictResolutionMode::Block)?;
        return Ok(false);
    }

    // Lire le blob de l'index conserve exactement les octets, y compris le dernier saut de ligne.
    let stage = if file.conflict_type == Some(ConflictType::DeletedByUs) {
        3
    } else {
        2
    };
    let entry = index
        .get_path(relative_path, stage)
        .ok_or_else(|| GitSvError::Other("Version du conflit introuvable dans l'index".into()))?;
    if entry.mode == 0o120000 {
        return Err(GitSvError::Other(
            "Résolvez les liens symboliques avec Git".into(),
        ));
    }
    let blob = repo.find_blob(entry.id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, blob.content())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(entry.mode & 0o777))?;
    }
    index.add_path(relative_path)?;
    index.write()?;
    Ok(false)
}
