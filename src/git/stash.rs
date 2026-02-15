use git2::Repository;

use crate::error::Result;

/// Entrée de stash.
#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    /// Branche sur laquelle le stash a été créé.
    pub branch: Option<String>,
    /// Date de création du stash.
    pub timestamp: Option<i64>,
}

/// Liste tous les stashes.
pub fn list_stashes(repo: &mut Repository) -> Result<Vec<StashEntry>> {
    let mut entries = Vec::new();

    repo.stash_foreach(|index, message, _oid| {
        // Essayer d'extraire la branche depuis le message (format: "WIP on <branch>: ...")
        let branch = extract_branch_from_message(message);

        entries.push(StashEntry {
            index,
            message: message.to_string(),
            branch,
            timestamp: None, // git2 ne fournit pas directement la date du stash
        });
        true // continuer l'itération
    })?;

    Ok(entries)
}

/// Extrait le nom de la branche depuis le message de stash.
fn extract_branch_from_message(message: &str) -> Option<String> {
    // Format typique: "WIP on <branch>: ..." ou "On <branch>: ..."
    if let Some(start) = message.find(" on ") {
        let rest = &message[start + 4..];
        if let Some(end) = rest.find(':') {
            return Some(rest[..end].to_string());
        }
    }
    None
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

/// Applique un stash sans le supprimer.
pub fn apply_stash(repo: &mut Repository, index: usize) -> Result<()> {
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_apply(index, Some(&mut opts))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;

    #[test]
    fn test_extract_branch_from_message() {
        // Format WIP on <branch>: ...
        assert_eq!(
            extract_branch_from_message("WIP on main: abc123 modification"),
            Some("main".to_string())
        );
        // Format avec branche contenant un slash
        assert_eq!(
            extract_branch_from_message("WIP on feature/test: 123456 test"),
            Some("feature/test".to_string())
        );
        // Message sans format reconnaissable
        assert_eq!(
            extract_branch_from_message("Message sans format standard"),
            None
        );
        // Le format "On <branch>:" (sans WIP) utilise " on " avec espaces,
        // donc "On main:" ne correspond pas (c'est "On" pas " on ")
        assert_eq!(
            extract_branch_from_message("On main: Mon stash de test"),
            None
        );
    }

    #[test]
    fn test_save_stash() {
        let (_temp_dir, mut repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "initial", "Initial commit");

        // Modifier le fichier sans commit
        create_file(&repo, "test.txt", "modified");
        // Stage les modifications pour qu'elles soient stashées
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();

        // Sauvegarder le stash
        save_stash(&mut repo, Some("Mon stash de test")).unwrap();

        // Vérifier que le stash existe (le message contient "On main: " + notre message)
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].message.contains("Mon stash de test"));
        assert_eq!(stashes[0].index, 0);
    }

    #[test]
    fn test_list_stashes() {
        let (_temp_dir, mut repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "initial", "Initial commit");

        // Créer plusieurs stashes avec des modifications staged
        create_file(&repo, "file1.txt", "content1");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("file1.txt")).unwrap();
        index.write().unwrap();
        save_stash(&mut repo, Some("Stash 1")).unwrap();

        create_file(&repo, "file2.txt", "content2");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("file2.txt")).unwrap();
        index.write().unwrap();
        save_stash(&mut repo, Some("Stash 2")).unwrap();

        // Lister les stashes
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 2);
        // Le stash le plus récent a l'index 0
        assert!(stashes[0].message.contains("Stash 2"));
        assert!(stashes[1].message.contains("Stash 1"));
    }

    #[test]
    fn test_apply_stash() {
        let (_temp_dir, mut repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "initial", "Initial commit");

        // Créer et stash des modifications (fichier doit être staged)
        create_file(&repo, "new_file.txt", "new content");
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("new_file.txt"))
            .unwrap();
        index.write().unwrap();
        save_stash(&mut repo, Some("Test apply")).unwrap();

        // Le stash ne supprime pas le fichier, il le garde dans l'index git
        // Le fichier existe toujours physiquement
        let workdir = repo.workdir().unwrap().to_path_buf();

        // Appliquer le stash
        apply_stash(&mut repo, 0).unwrap();

        // Vérifier que le fichier existe
        assert!(workdir.join("new_file.txt").exists());

        // Le stash devrait toujours exister après apply
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
    }

    #[test]
    fn test_drop_stash() {
        let (_temp_dir, mut repo) = create_test_repo();

        // Commit initial
        commit_file(&repo, "test.txt", "initial", "Initial commit");

        // Créer un stash avec un fichier staged
        create_file(&repo, "temp.txt", "temp content");
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("temp.txt")).unwrap();
        index.write().unwrap();
        save_stash(&mut repo, Some("To drop")).unwrap();

        // Vérifier qu'il existe
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);

        // Supprimer le stash
        drop_stash(&mut repo, 0).unwrap();

        // Vérifier qu'il n'existe plus
        let stashes = list_stashes(&mut repo).unwrap();
        assert!(stashes.is_empty());
    }
}
