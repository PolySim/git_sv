use git2::Repository;

use crate::error::Result;

/// Entrée de stash.
#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

/// Liste tous les stashes.
pub fn list_stashes(repo: &mut Repository) -> Result<Vec<StashEntry>> {
    let mut entries = Vec::new();

    repo.stash_foreach(|index, message, _oid| {
        entries.push(StashEntry {
            index,
            message: message.to_string(),
        });
        true // continuer l'itération
    })?;

    Ok(entries)
}

/// Sauvegarde le working directory dans un stash.
pub fn save_stash(repo: &mut Repository, message: Option<&str>) -> Result<()> {
    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("git_sv", "git_sv@local"))?;

    let msg = message.unwrap_or("Stash créé par git_sv");
    repo.stash_save(&sig, msg, None)?;
    Ok(())
}

/// Applique et supprime le stash à l'index donné.
pub fn pop_stash(repo: &mut Repository, index: usize) -> Result<()> {
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_pop(index, Some(&mut opts))?;
    Ok(())
}

/// Supprime le stash à l'index donné sans l'appliquer.
pub fn drop_stash(repo: &mut Repository, index: usize) -> Result<()> {
    repo.stash_drop(index)?;
    Ok(())
}
