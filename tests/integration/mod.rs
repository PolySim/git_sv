//! Tests d'intégration pour les workflows complets.

use git2::Repository;
use std::path::Path;
use tempfile::TempDir;

/// Structure helper pour créer des repos de test.
pub struct TestRepo {
    _temp_dir: TempDir,
    pub repo: Repository,
}

impl TestRepo {
    /// Crée un nouveau repo de test.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = git2::Repository::init_opts(temp_dir.path(), &opts).unwrap();

        // Configurer git
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }

        // Commit initial
        {
            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let mut index = repo.index().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        Self {
            _temp_dir: temp_dir,
            repo,
        }
    }

    /// Crée un fichier dans le repo.
    pub fn create_file(&self, path: &str, content: &str) {
        let workdir = self.repo.workdir().unwrap();
        let full_path = workdir.join(path);

        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        std::fs::write(&full_path, content).unwrap();
    }

    /// Stage un fichier.
    pub fn stage_file(&self, path: &str) {
        let mut index = self.repo.index().unwrap();
        index.add_path(Path::new(path)).unwrap();
        index.write().unwrap();
    }

    /// Commit les changements stagés.
    pub fn commit(&self, message: &str) -> git2::Oid {
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let mut index = self.repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = self.repo.find_tree(tree_oid).unwrap();

        let parent_commit = self
            .repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .and_then(|oid| self.repo.find_commit(oid).ok());

        let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

        self.repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    /// Retourne le message du dernier commit.
    pub fn last_commit_message(&self) -> Option<String> {
        self.repo
            .head()
            .ok()?
            .peel_to_commit()
            .ok()?
            .message()
            .map(|s| s.to_string())
    }

    /// Liste les branches locales.
    pub fn list_branches(&self) -> Vec<String> {
        self.repo
            .branches(Some(git2::BranchType::Local))
            .unwrap()
            .filter_map(|b| {
                b.ok()
                    .and_then(|(branch, _)| branch.name().ok()?.map(|s| s.to_string()))
            })
            .collect()
    }

    /// Retourne la branche courante.
    pub fn current_branch(&self) -> String {
        self.repo
            .head()
            .unwrap()
            .shorthand()
            .unwrap_or("HEAD")
            .to_string()
    }

    /// Crée une nouvelle branche.
    pub fn create_branch(&self, name: &str) {
        let head = self.repo.head().unwrap().peel_to_commit().unwrap();
        self.repo.branch(name, &head, false).unwrap();
    }

    /// Checkout une branche.
    pub fn checkout_branch(&self, name: &str) {
        let branch = self
            .repo
            .find_branch(name, git2::BranchType::Local)
            .unwrap();
        let reference = branch.get();
        self.repo.set_head(reference.name().unwrap()).unwrap();
    }

    /// Liste les stashes.
    pub fn list_stashes(&mut self) -> Vec<(usize, String)> {
        let mut stashes = Vec::new();
        self.repo
            .stash_foreach(|index, message, _| {
                stashes.push((index, message.to_string()));
                true
            })
            .unwrap();
        stashes
    }
}

#[test]
fn test_full_commit_workflow() {
    let test_repo = TestRepo::new();

    // Créer un fichier
    test_repo.create_file("test.txt", "Hello, World!");

    // Stage le fichier
    test_repo.stage_file("test.txt");

    // Vérifier que le fichier est bien dans l'index
    let status = test_repo.repo.status_file(Path::new("test.txt")).unwrap();
    assert!(status.contains(git2::Status::INDEX_NEW));

    // Commit
    test_repo.commit("Test commit");

    // Vérifier le commit
    let last_msg = test_repo.last_commit_message().unwrap();
    assert!(last_msg.contains("Test commit"));

    // Vérifier que le working directory est propre
    let mut opts = git2::StatusOptions::new();
    let statuses = test_repo.repo.statuses(Some(&mut opts)).unwrap();
    assert!(statuses.is_empty());
}

#[test]
fn test_branch_create_and_checkout() {
    let test_repo = TestRepo::new();

    // Créer une nouvelle branche
    test_repo.create_branch("feature/test");

    // Vérifier que la branche existe
    let branches = test_repo.list_branches();
    assert!(branches.contains(&"feature/test".to_string()));

    // Checkout sur la nouvelle branche
    test_repo.checkout_branch("feature/test");

    // Vérifier qu'on est sur la nouvelle branche
    assert_eq!(test_repo.current_branch(), "feature/test");
}

#[test]
fn test_stash_save_and_apply() {
    let mut test_repo = TestRepo::new();

    // Créer un fichier tracked et le modifier
    test_repo.create_file("test.txt", "initial content");
    test_repo.stage_file("test.txt");
    test_repo.commit("Add test file");

    // Modifier le fichier (maintenant tracked)
    test_repo.create_file("test.txt", "modified content");

    // Vérifier qu'il y a des modifications
    {
        let mut opts = git2::StatusOptions::new();
        let statuses = test_repo.repo.statuses(Some(&mut opts)).unwrap();
        assert!(!statuses.is_empty(), "Devrait avoir des modifications");
    }

    // Sauvegarder le stash (inclure les fichiers tracked modifiés)
    let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
    test_repo
        .repo
        .stash_save(
            &sig,
            "stash test",
            Some(git2::StashFlags::INCLUDE_UNTRACKED),
        )
        .unwrap();

    // Vérifier que le working directory est propre
    {
        let mut opts = git2::StatusOptions::new();
        let statuses = test_repo.repo.statuses(Some(&mut opts)).unwrap();
        assert!(
            statuses.is_empty(),
            "Le working directory devrait être propre"
        );
    }

    // Vérifier qu'il y a un stash
    let stashes = test_repo.list_stashes();
    assert_eq!(stashes.len(), 1, "Devrait avoir un stash");

    // Appliquer le stash
    test_repo.repo.stash_pop(0, None).unwrap();

    // Vérifier que les modifications sont revenues
    {
        let mut opts = git2::StatusOptions::new();
        let statuses = test_repo.repo.statuses(Some(&mut opts)).unwrap();
        assert!(
            !statuses.is_empty(),
            "Les modifications devraient être revenues"
        );
    }
}
