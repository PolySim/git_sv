//! Opérations sur les commits : création, amendement, log.

use git2::{Oid, Repository, Signature};

use crate::error::Result;

/// Informations essentielles d'un commit.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: Oid,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub parents: Vec<Oid>,
    /// Chemins des fichiers modifiés par ce commit (optionnel, chargé à la demande).
    pub changed_paths: Option<Vec<String>>,
}

impl CommitInfo {
    /// Crée un CommitInfo depuis un git2::Commit.
    pub fn from_git2_commit(commit: &git2::Commit) -> Self {
        let message = commit.summary().ok().flatten().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("Inconnu").to_string();
        let email = commit.author().email().unwrap_or("").to_string();
        let timestamp = commit.time().seconds();
        let parents = commit.parent_ids().collect();

        Self {
            oid: commit.id(),
            message,
            author,
            email,
            timestamp,
            parents,
            changed_paths: None,
        }
    }

    /// Charge les chemins modifiés pour ce commit depuis le repository.
    pub fn load_changed_paths(&mut self, repo: &Repository) -> crate::error::Result<()> {
        self.changed_paths = Some(super::diff::commit_changed_paths(repo, self.oid)?);
        Ok(())
    }

    /// Retourne les chemins modifiés, en les chargeant si nécessaire.
    #[allow(dead_code)]
    pub fn changed_paths(&mut self, repo: &Repository) -> crate::error::Result<&[String]> {
        if self.changed_paths.is_none() {
            self.load_changed_paths(repo)?;
        }
        Ok(self.changed_paths.as_ref().unwrap().as_slice())
    }

    /// Retourne le hash court (7 caractères).
    #[allow(dead_code)]
    pub fn short_hash(&self) -> String {
        self.oid.to_string()[..7].to_string()
    }
}

/// Crée un commit avec le message donné sur l'index courant.
pub fn create_commit(repo: &Repository, message: &str) -> Result<Oid> {
    if repo.state() == git2::RepositoryState::Merge {
        // Le commit de résolution doit conserver tous les parents du merge.
        super::conflict::finalize_merge(repo, message)?;
        return Ok(repo.head()?.peel_to_commit()?.id());
    }
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("git_sv", "git_sv@local"))?;

    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent_commit = match repo.head() {
        Ok(head) => {
            let oid = head.target().ok_or_else(|| {
                crate::error::GitSvError::Other("HEAD ne pointe pas vers un commit".into())
            })?;
            Some(repo.find_commit(oid)?)
        }
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => None,
        Err(error) => return Err(error.into()),
    };

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

    Ok(oid)
}

/// Amende le dernier commit avec le message donné et l'index courant.
pub fn amend_commit(repo: &Repository, message: &str) -> Result<Oid> {
    if repo.state() == git2::RepositoryState::Merge {
        return Err(crate::error::GitSvError::Other(
            "Impossible d'amender un commit pendant une fusion ; créez le commit de résolution"
                .into(),
        ));
    }
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("git_sv", "git_sv@local"))?;

    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Récupérer le commit HEAD
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;

    // Amender le commit (remplace HEAD)
    let oid = head_commit.amend(
        Some("HEAD"),  // Mettre à jour HEAD
        None,          // Ne pas changer l'auteur
        Some(&sig),    // Actualiser l'identité et la date du committer
        None,          // Ne pas changer l'encodage
        Some(message), // Nouveau message
        Some(&tree),   // Nouveau tree
    )?;

    Ok(oid)
}

/// Stage un fichier dans l'index.
pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    let status = repo.status_file(std::path::Path::new(path))?;
    update_index(repo, |index| {
        if status.contains(git2::Status::WT_DELETED) {
            index.remove_path(std::path::Path::new(path))
        } else {
            index.add_path(std::path::Path::new(path))
        }
    })
}

/// Stage tous les fichiers modifiés.
pub fn stage_all(repo: &Repository) -> Result<()> {
    update_index(repo, |index| {
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
    })
}

fn update_index(
    repo: &Repository,
    edit: impl FnOnce(&mut git2::Index) -> std::result::Result<(), git2::Error>,
) -> Result<()> {
    let mut index = repo.index()?;
    let result = edit(&mut index).and_then(|()| index.write());
    if result.is_err() {
        // Un write refusé ne doit pas laisser des changements indexés en mémoire.
        let _ = index.read(true);
    }
    result.map_err(Into::into)
}

/// Unstage un fichier (le retirer de l'index, revenir à HEAD).
pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    let head = head_commit_or_unborn(repo)?;
    if head.is_none() {
        return update_index(repo, |index| index.remove_path(std::path::Path::new(path)));
    }
    // reset_default interprète les métacaractères du nom comme un pathspec.
    let mut command = std::process::Command::new("git");
    command
        .args([
            "--literal-pathspecs",
            "restore",
            "--staged",
            "--source=HEAD",
            "--",
            path,
        ])
        .current_dir(repo.workdir().unwrap_or(repo.path()));
    let output = crate::git::helpers::run_command_with_timeout(
        command,
        "désindexation",
        std::time::Duration::from_secs(30),
    )?;
    if !output.status.success() {
        return Err(crate::error::GitSvError::OperationFailed {
            operation: "désindexation",
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    repo.index()?.read(true)?;
    Ok(())
}

/// Unstage tous les fichiers sans modifier le worktree ni l'opération Git en cours.
pub fn unstage_all(repo: &Repository) -> Result<()> {
    let head = head_commit_or_unborn(repo)?;
    if let Some(head) = head {
        let tree = head.tree()?;
        update_index(repo, |index| index.read_tree(&tree))
    } else {
        update_index(repo, |index| index.clear())
    }
}

fn head_commit_or_unborn(repo: &Repository) -> Result<Option<git2::Commit<'_>>> {
    match repo.head() {
        Ok(head) => Ok(Some(head.peel_to_commit()?)),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Cherry-pick un commit sur la branche courante.
#[allow(dead_code)]
pub fn cherry_pick(repo: &Repository, commit_oid: Oid) -> Result<()> {
    let commit = repo.find_commit(commit_oid)?;

    // Effectuer le cherry-pick
    repo.cherrypick(&commit, None)?;

    // Vérifier s'il y a des conflits
    let mut index = repo.index()?;
    if index.has_conflicts() {
        return Err(crate::error::GitSvError::Other(
            "Cherry-pick a échoué: conflits détectés".into(),
        ));
    }

    // Si pas de conflits, créer le commit
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("git_sv", "git_sv@local"))?;

    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let head = repo.head()?;
    let parent_commit = head.peel_to_commit()?;

    let message = format!(
        "{}\n\n(cherry picked from commit {})",
        commit.message().unwrap_or(""),
        commit_oid
    );

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent_commit])?;

    // Nettoyer l'état de cherry-pick
    repo.cleanup_state()?;

    Ok(())
}

/// Reset la branche courante vers le commit spécifié.
pub fn reset_to_commit(repo: &Repository, oid: Oid, reset_type: git2::ResetType) -> Result<()> {
    let commit = repo.find_commit(oid)?;
    let object = commit.as_object();
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force(); // Nécessaire pour Hard reset
    repo.reset(object, reset_type, Some(&mut checkout_builder))?;
    Ok(())
}

/// Cherry-pick avec résultat typé pour gérer les conflits.
pub fn cherry_pick_with_result(
    repo: &Repository,
    commit_oid: Oid,
) -> crate::error::Result<crate::git::conflict::MergeResult> {
    use crate::git::conflict::{list_conflict_files, MergeResult};

    let commit = repo.find_commit(commit_oid)?;

    // Effectuer le cherry-pick
    repo.cherrypick(&commit, None)?;

    // Vérifier s'il y a des conflits
    let mut index = repo.index()?;
    if index.has_conflicts() {
        // Lister les fichiers en conflit
        let conflict_files = list_conflict_files(repo)?;
        return Ok(MergeResult::Conflicts(conflict_files));
    }

    // Si pas de conflits, créer le commit
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("git_sv", "git_sv@local"))?;

    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let head = repo.head()?;
    let parent_commit = head.peel_to_commit()?;

    let message = format!(
        "{}\n\n(cherry picked from commit {})",
        commit.message().unwrap_or(""),
        commit_oid
    );

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent_commit])?;

    // Nettoyer l'état de cherry-pick
    repo.cleanup_state()?;

    Ok(MergeResult::Success)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::tests::test_utils::*;
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn staging_does_not_import_contents_through_a_replaced_parent_symlink() {
        for all in [false, true] {
            let (temp, repo) = create_test_repo();
            commit_file(&repo, "folder/file.txt", "base\n", "base");
            std::fs::remove_dir_all(temp.path().join("folder")).unwrap();
            let outside = tempfile::TempDir::new().unwrap();
            std::fs::write(outside.path().join("file.txt"), b"external secret\n").unwrap();
            std::os::unix::fs::symlink(outside.path(), temp.path().join("folder")).unwrap();
            let _ = if all {
                stage_all(&repo)
            } else {
                stage_file(&repo, "folder/file.txt")
            };
            for entry in repo.index().unwrap().iter() {
                assert_ne!(
                    repo.find_blob(entry.id).unwrap().content(),
                    b"external secret\n"
                );
            }
            assert_eq!(
                std::fs::read(outside.path().join("file.txt")).unwrap(),
                b"external secret\n"
            );
        }
    }

    #[test]
    fn unstage_file_targets_literal_metacharacters_without_touching_siblings() {
        let (temp, repo) = create_test_repo();
        for path in ["[a].txt", "a.txt"] {
            commit_file(&repo, path, "base\n", "initial");
            std::fs::write(temp.path().join(path), "staged\n").unwrap();
            stage_file(&repo, path).unwrap();
        }
        // Re-stager le premier fichier, car le deuxième commit a utilisé son index.
        std::fs::write(temp.path().join("[a].txt"), "selected staged\n").unwrap();
        stage_file(&repo, "[a].txt").unwrap();
        let sibling = repo
            .index()
            .unwrap()
            .get_path(Path::new("a.txt"), 0)
            .unwrap()
            .id;
        unstage_file(&repo, "[a].txt").unwrap();
        let index = repo.index().unwrap();
        assert_eq!(index.get_path(Path::new("a.txt"), 0).unwrap().id, sibling);
        let expected = repo
            .head()
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .get_path(Path::new("[a].txt"))
            .unwrap()
            .id();
        assert_eq!(
            index.get_path(Path::new("[a].txt"), 0).unwrap().id,
            expected
        );
        assert_eq!(
            std::fs::read(temp.path().join("[a].txt")).unwrap(),
            b"selected staged\n"
        );
    }

    #[test]
    fn ordinary_commit_finishes_merge_with_both_parents() {
        let (_temp, repo) = create_test_repo();
        let base = commit_file(&repo, "base.txt", "base\n", "base");
        repo.branch("other", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        let ours = commit_file(&repo, "ours.txt", "ours\n", "ours");
        repo.set_head("refs/heads/other").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        let theirs = commit_file(&repo, "theirs.txt", "theirs\n", "theirs");
        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        let annotated = repo.find_annotated_commit(theirs).unwrap();
        repo.merge(&[&annotated], None, None).unwrap();
        drop(annotated);
        assert_eq!(repo.state(), git2::RepositoryState::Merge);
        let index_before = std::fs::read(repo.path().join("index")).unwrap();
        assert!(amend_commit(&repo, "Amend interdit").is_err());
        assert_eq!(repo.head().unwrap().target(), Some(ours));
        assert_eq!(
            std::fs::read(repo.path().join("index")).unwrap(),
            index_before
        );
        let oid = create_commit(&repo, "Merge résolu 🚀").unwrap();
        let commit = repo.find_commit(oid).unwrap();
        assert_eq!(commit.parent_ids().collect::<Vec<_>>(), vec![ours, theirs]);
        assert_eq!(repo.state(), git2::RepositoryState::Clean);
        assert!(!repo.path().join("MERGE_HEAD").exists());
    }

    #[test]
    fn unresolved_merge_commit_preserves_head_index_and_merge_state() {
        let (_temp, repo) = create_test_repo();
        let base = commit_file(&repo, "conflict.txt", "base\n", "base");
        repo.branch("other", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        let ours = commit_file(&repo, "conflict.txt", "ours\n", "ours");
        repo.set_head("refs/heads/other").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        let theirs = commit_file(&repo, "conflict.txt", "theirs\n", "theirs");
        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        let annotated = repo.find_annotated_commit(theirs).unwrap();
        repo.merge(&[&annotated], None, None).unwrap();
        let before_index = std::fs::read(repo.path().join("index")).unwrap();
        let before_merge_head = std::fs::read(repo.path().join("MERGE_HEAD")).unwrap();
        assert!(create_commit(&repo, "Impossible").is_err());
        assert_eq!(repo.head().unwrap().target(), Some(ours));
        assert!(repo.index().unwrap().has_conflicts());
        assert_eq!(
            std::fs::read(repo.path().join("index")).unwrap(),
            before_index
        );
        assert_eq!(
            std::fs::read(repo.path().join("MERGE_HEAD")).unwrap(),
            before_merge_head
        );
    }

    #[test]
    fn rejected_index_writes_leave_disk_and_cached_staging_unchanged() {
        for stage in [false, true] {
            let (temp, repo) = create_test_repo();
            commit_file(&repo, "file.txt", "base\n", "base");
            std::fs::write(temp.path().join("file.txt"), "staged\n").unwrap();
            stage_file(&repo, "file.txt").unwrap();
            std::fs::write(temp.path().join("file.txt"), "unstaged\n").unwrap();
            let before = repo.index().unwrap().write_tree().unwrap();
            let before_bytes = std::fs::read(repo.path().join("index")).unwrap();
            std::fs::write(repo.path().join("index.lock"), b"lock").unwrap();
            let result = if stage {
                stage_file(&repo, "file.txt")
            } else {
                unstage_file(&repo, "file.txt")
            };
            assert!(result.is_err());
            assert_eq!(
                std::fs::read(repo.path().join("index")).unwrap(),
                before_bytes
            );
            assert_eq!(repo.index().unwrap().write_tree().unwrap(), before);
            assert_eq!(
                std::fs::read(temp.path().join("file.txt")).unwrap(),
                b"unstaged\n"
            );
        }
    }

    #[test]
    fn test_commit_info_from_git2_commit() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un commit
        let oid = commit_file(&repo, "test.txt", "Hello World", "Test commit message");
        let commit = repo.find_commit(oid).unwrap();

        let info = CommitInfo::from_git2_commit(&commit);

        assert_eq!(info.message, "Test commit message");
        assert_eq!(info.author, "Test User");
        assert_eq!(info.email, "test@example.com");
        assert!(info.timestamp > 0);
        assert!(info.parents.is_empty()); // Premier commit
    }

    #[test]
    fn test_commit_info_short_hash() {
        let (_temp_dir, repo) = create_test_repo();

        let oid = commit_file(&repo, "test.txt", "content", "Test");
        let commit = repo.find_commit(oid).unwrap();
        let info = CommitInfo::from_git2_commit(&commit);

        let short = info.short_hash();
        assert_eq!(short.len(), 7);
        assert!(oid.to_string().starts_with(&short));
    }

    #[test]
    fn test_stage_file() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un fichier
        create_file(&repo, "test.txt", "Hello");

        // Stage le fichier
        stage_file(&repo, "test.txt").unwrap();

        // Vérifier qu'il est dans l'index
        let index = repo.index().unwrap();
        let entries: Vec<_> = index.iter().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_stage_all() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer plusieurs fichiers
        create_file(&repo, "file1.txt", "content1");
        create_file(&repo, "file2.txt", "content2");

        // Stage tous les fichiers
        stage_all(&repo).unwrap();

        // Vérifier qu'ils sont dans l'index
        let index = repo.index().unwrap();
        let entries: Vec<_> = index.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_unstage_file() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer et commiter un fichier
        commit_file(&repo, "test.txt", "Initial content", "Initial commit");

        // Modifier le fichier et stage
        create_file(&repo, "test.txt", "Modified content");
        stage_file(&repo, "test.txt").unwrap();

        // Vérifier qu'il est staged (modification présente dans l'index)
        let index = repo.index().unwrap();
        assert_eq!(index.iter().count(), 1);

        // Unstage le fichier - retour à HEAD
        unstage_file(&repo, "test.txt").unwrap();

        // Après unstage, le fichier devrait correspondre à HEAD
        // Donc il n'y a plus de diff entre index et HEAD
        let statuses = repo.statuses(None).unwrap();
        // Le fichier ne devrait plus être dans l'état "staged modified"
        let mut found_staged = false;
        for entry in statuses.iter() {
            if entry.path().ok() == Some("test.txt") {
                let status = entry.status();
                if status.contains(git2::Status::INDEX_MODIFIED) {
                    found_staged = true;
                }
            }
        }
        assert!(!found_staged, "Le fichier ne devrait plus être staged");
    }

    #[test]
    fn test_stage_deleted_file() {
        let (temp_dir, repo) = create_test_repo();

        commit_file(&repo, "deleted.txt", "Initial content", "Initial commit");

        std::fs::remove_file(temp_dir.path().join("deleted.txt")).unwrap();

        stage_file(&repo, "deleted.txt").unwrap();

        let statuses = repo.statuses(None).unwrap();
        let deleted_status = statuses
            .iter()
            .find(|entry| entry.path().ok() == Some("deleted.txt"))
            .map(|entry| entry.status())
            .expect("Le fichier supprimé devrait apparaître dans le status");

        assert!(deleted_status.contains(git2::Status::INDEX_DELETED));
    }

    #[test]
    fn test_create_commit() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un fichier et l'ajouter à l'index
        create_file(&repo, "test.txt", "content");
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        // Créer un commit
        let oid = create_commit(&repo, "My commit message").unwrap();

        // Vérifier que le commit existe
        let commit = repo.find_commit(oid).unwrap();
        assert_eq!(commit.summary().unwrap(), Some("My commit message"));
    }

    #[test]
    fn test_reset_to_commit_soft() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un premier commit
        let first_oid = commit_file(&repo, "test.txt", "Initial content", "First commit");

        // Créer un second commit
        let second_oid = commit_file(&repo, "test.txt", "Modified content", "Second commit");

        // Vérifier que HEAD pointe vers le second commit
        let head = repo.head().unwrap();
        let head_oid = head.target().unwrap();
        assert_eq!(head_oid, second_oid);

        // Reset soft vers le premier commit
        reset_to_commit(&repo, first_oid, git2::ResetType::Soft).unwrap();

        // Vérifier que HEAD pointe maintenant vers le premier commit
        let head = repo.head().unwrap();
        let head_oid = head.target().unwrap();
        assert_eq!(head_oid, first_oid);

        // En mode soft, les modifications devraient être dans l'index
        let index = repo.index().unwrap();
        assert_eq!(index.iter().count(), 1);
    }

    #[test]
    fn test_reset_to_commit_hard() {
        let (_temp_dir, repo) = create_test_repo();

        // Créer un premier commit
        let first_oid = commit_file(&repo, "test.txt", "Initial content", "First commit");

        // Créer un second commit
        let second_oid = commit_file(&repo, "test.txt", "Modified content", "Second commit");

        // Vérifier que HEAD pointe vers le second commit
        let head = repo.head().unwrap();
        let head_oid = head.target().unwrap();
        assert_eq!(head_oid, second_oid);

        // Reset hard vers le premier commit
        reset_to_commit(&repo, first_oid, git2::ResetType::Hard).unwrap();

        // Vérifier que HEAD pointe maintenant vers le premier commit
        let head = repo.head().unwrap();
        let head_oid = head.target().unwrap();
        assert_eq!(head_oid, first_oid);

        // En mode hard, l'index devrait correspondre au commit cible
        // Donc le fichier devrait avoir le contenu initial
        let index = repo.index().unwrap();
        assert_eq!(index.iter().count(), 1);
    }

    #[test]
    fn unstage_before_first_commit_preserves_working_files() {
        let (_temp, repo) = create_test_repo();
        std::fs::write(repo.workdir().unwrap().join("file.txt"), "content").unwrap();
        stage_file(&repo, "file.txt").unwrap();
        unstage_file(&repo, "file.txt").unwrap();
        assert!(repo.index().unwrap().is_empty());
        stage_file(&repo, "file.txt").unwrap();
        unstage_all(&repo).unwrap();
        assert!(repo.index().unwrap().is_empty());
        assert_eq!(
            std::fs::read_to_string(repo.workdir().unwrap().join("file.txt")).unwrap(),
            "content"
        );
    }

    #[test]
    fn amend_updates_committer_and_preserves_author() {
        let (_temp, repo) = create_test_repo();
        let initial = commit_file(&repo, "file", "content", "initial");
        let author = repo
            .find_commit(initial)
            .unwrap()
            .author()
            .name()
            .unwrap()
            .to_string();
        repo.config()
            .unwrap()
            .set_str("user.name", "New Committer")
            .unwrap();
        let amended = amend_commit(&repo, "amended").unwrap();
        let commit = repo.find_commit(amended).unwrap();
        assert_eq!(commit.author().name().unwrap(), author);
        assert_eq!(commit.committer().name().unwrap(), "New Committer");
    }
}
