//! Resolution de conflits de merge : detection, modes (fichier/bloc/ligne), finalisation.

mod content;
mod parse;
mod repo_state;
mod resolve;
mod types;

pub use content::{
    all_sections_resolved, apply_resolved_content, generate_resolved_content_with_source,
};
pub use parse::{list_conflict_files, parse_conflict_file};
pub use repo_state::{abort_merge, finalize_merge, get_current_branch_name, is_merging};
pub use resolve::{resolve_file_with_strategy, resolve_special_file};
pub use types::{
    ConflictFile, ConflictResolution, ConflictResolutionMode, ConflictSection, ConflictType,
    LineLevelResolution, LineSource, MergeFile, MergeResult, ResolvedLine,
};

/// Résout un chemin de conflit dans le working tree, sans suivre de lien symbolique.
pub fn worktree_path(
    repo: &git2::Repository,
    path: &str,
) -> crate::error::Result<std::path::PathBuf> {
    use std::path::Component;

    let mut resolved = repo
        .workdir()
        .ok_or_else(|| crate::error::GitSvError::Other("Working tree Git introuvable".into()))?
        .to_path_buf();
    for component in std::path::Path::new(path).components() {
        let Component::Normal(name) = component else {
            return Err(crate::error::GitSvError::Other(
                "Chemin de conflit invalide".into(),
            ));
        };
        resolved.push(name);
        if std::fs::symlink_metadata(&resolved)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(crate::error::GitSvError::Other(
                "Résolvez les liens symboliques avec Git".into(),
            ));
        }
    }
    if path.is_empty() {
        return Err(crate::error::GitSvError::Other(
            "Chemin de conflit vide".into(),
        ));
    }
    Ok(resolved)
}
