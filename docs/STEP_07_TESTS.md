# STEP 07 - Tests Unitaires et d'Intégration

**Priorité**: 🟡 Moyenne  
**Effort estimé**: 4-6 heures  
**Risque**: Faible  
**Prérequis**: STEP_01 à STEP_06 complétés

---

## Objectif

Améliorer la couverture de tests en ajoutant :
1. Tests unitaires pour les handlers (actuellement 0%)
2. Tests pour les composants UI (snapshot testing)
3. Tests d'intégration pour les workflows complets
4. Mocking des dépendances git

---

## 1. État actuel des tests

### Couverture existante ✓

| Module | Tests | Couverture |
|--------|-------|------------|
| `git/repo.rs` | 5 | Bonne |
| `git/branch.rs` | 5 | Bonne |
| `git/commit.rs` | 3 | Moyenne |
| `git/stash.rs` | 4 | Bonne |
| `git/diff.rs` | 5 | Bonne |
| `git/graph.rs` | 4 | Bonne |
| `git/blame.rs` | 1 | Faible |
| `git/merge.rs` | 1 | Faible |
| `git/search.rs` | 4 | Bonne |
| `git/discard.rs` | 2 | Moyenne |
| `utils/time.rs` | 9 | Excellente |

### Manques identifiés ✗

| Module | Tests | Priorité |
|--------|-------|----------|
| `handler/*` | 0 | Haute |
| `ui/*` | 0 | Moyenne |
| `state/*` | 0 | Moyenne |
| `app.rs` | 0 | Basse |

---

## 2. Infrastructure de test

### 2.1 Module de test helpers

#### Fichier: `src/test_utils/mod.rs`

```rust
//! Utilitaires de test partagés.

#[cfg(test)]
pub mod mock_repo;

#[cfg(test)]
pub mod test_state;

#[cfg(test)]
pub mod assertions;
```

#### Fichier: `src/test_utils/mock_repo.rs`

```rust
//! Mock repository pour les tests sans filesystem.

use std::collections::HashMap;
use git2::Oid;

/// Mock d'un repository git pour les tests.
#[derive(Default)]
pub struct MockRepo {
    pub branches: Vec<MockBranch>,
    pub commits: Vec<MockCommit>,
    pub staged_files: Vec<String>,
    pub unstaged_files: Vec<String>,
    pub current_branch: Option<String>,
}

pub struct MockBranch {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
}

pub struct MockCommit {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub parent_oids: Vec<String>,
}

impl MockRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_branch(mut self, name: &str, is_head: bool) -> Self {
        self.branches.push(MockBranch {
            name: name.to_string(),
            is_head,
            is_remote: false,
        });
        if is_head {
            self.current_branch = Some(name.to_string());
        }
        self
    }

    pub fn with_commit(mut self, oid: &str, message: &str) -> Self {
        self.commits.push(MockCommit {
            oid: oid.to_string(),
            message: message.to_string(),
            author: "Test Author".to_string(),
            parent_oids: vec![],
        });
        self
    }

    pub fn with_staged(mut self, file: &str) -> Self {
        self.staged_files.push(file.to_string());
        self
    }

    pub fn with_unstaged(mut self, file: &str) -> Self {
        self.unstaged_files.push(file.to_string());
        self
    }
}

/// Trait pour permettre le mocking dans les handlers.
pub trait RepositoryLike {
    fn current_branch(&self) -> Option<&str>;
    fn list_branches(&self) -> Vec<String>;
    fn staged_files(&self) -> &[String];
    fn unstaged_files(&self) -> &[String];
}

impl RepositoryLike for MockRepo {
    fn current_branch(&self) -> Option<&str> {
        self.current_branch.as_deref()
    }
    
    fn list_branches(&self) -> Vec<String> {
        self.branches.iter().map(|b| b.name.clone()).collect()
    }
    
    fn staged_files(&self) -> &[String] {
        &self.staged_files
    }
    
    fn unstaged_files(&self) -> &[String] {
        &self.unstaged_files
    }
}
```

#### Fichier: `src/test_utils/test_state.rs`

```rust
//! Création de states de test.

use crate::state::{AppState, ViewMode, StagingState, BranchesViewState};
use crate::state::selection::ListSelection;

/// Builder pour créer des AppState de test.
pub struct TestStateBuilder {
    view_mode: ViewMode,
    current_branch: Option<String>,
    staged_count: usize,
    unstaged_count: usize,
}

impl Default for TestStateBuilder {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Graph,
            current_branch: Some("main".to_string()),
            staged_count: 0,
            unstaged_count: 0,
        }
    }
}

impl TestStateBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view_mode(mut self, mode: ViewMode) -> Self {
        self.view_mode = mode;
        self
    }

    pub fn branch(mut self, name: &str) -> Self {
        self.current_branch = Some(name.to_string());
        self
    }

    pub fn staged_files(mut self, count: usize) -> Self {
        self.staged_count = count;
        self
    }

    pub fn unstaged_files(mut self, count: usize) -> Self {
        self.unstaged_count = count;
        self
    }

    /// Construit un AppState minimal pour les tests.
    /// Note: Nécessite un vrai repo ou un mock selon le contexte.
    pub fn build_minimal(self) -> MinimalTestState {
        MinimalTestState {
            view_mode: self.view_mode,
            current_branch: self.current_branch,
            staging_state: StagingState::default(),
            branches_view_state: BranchesViewState::default(),
        }
    }
}

/// État minimal pour les tests unitaires (sans repo réel).
pub struct MinimalTestState {
    pub view_mode: ViewMode,
    pub current_branch: Option<String>,
    pub staging_state: StagingState,
    pub branches_view_state: BranchesViewState,
}
```

---

## 3. Tests des handlers

### 3.1 Tests du NavigationHandler

#### Fichier: `src/handler/navigation.rs` (section tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::selection::ListSelection;
    use crate::test_utils::test_state::TestStateBuilder;

    #[test]
    fn test_move_up_in_graph_view() {
        let mut state = create_test_state_with_graph(5);
        state.graph.select(3);  // Position initiale
        
        let handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };
        
        handler.handle(&mut ctx, NavigationAction::MoveUp).unwrap();
        
        assert_eq!(state.graph.selected_index(), 2);
    }

    #[test]
    fn test_move_up_at_top_stays_at_top() {
        let mut state = create_test_state_with_graph(5);
        state.graph.select(0);
        
        let handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };
        
        handler.handle(&mut ctx, NavigationAction::MoveUp).unwrap();
        
        assert_eq!(state.graph.selected_index(), 0);
    }

    #[test]
    fn test_move_down_in_graph_view() {
        let mut state = create_test_state_with_graph(5);
        state.graph.select(2);
        
        let handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };
        
        handler.handle(&mut ctx, NavigationAction::MoveDown).unwrap();
        
        assert_eq!(state.graph.selected_index(), 3);
    }

    #[test]
    fn test_move_down_at_bottom_stays_at_bottom() {
        let mut state = create_test_state_with_graph(5);
        state.graph.select(4);  // Dernier élément
        
        let handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };
        
        handler.handle(&mut ctx, NavigationAction::MoveDown).unwrap();
        
        assert_eq!(state.graph.selected_index(), 4);
    }

    #[test]
    fn test_page_up() {
        let mut state = create_test_state_with_graph(20);
        state.graph.set_visible_height(5);
        state.graph.select(15);
        
        let handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };
        
        handler.handle(&mut ctx, NavigationAction::PageUp).unwrap();
        
        assert_eq!(state.graph.selected_index(), 10);
    }

    #[test]
    fn test_go_top() {
        let mut state = create_test_state_with_graph(20);
        state.graph.select(15);
        
        let handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };
        
        handler.handle(&mut ctx, NavigationAction::GoTop).unwrap();
        
        assert_eq!(state.graph.selected_index(), 0);
    }

    #[test]
    fn test_go_bottom() {
        let mut state = create_test_state_with_graph(20);
        state.graph.select(5);
        
        let handler = NavigationHandler;
        let mut ctx = HandlerContext { state: &mut state };
        
        handler.handle(&mut ctx, NavigationAction::GoBottom).unwrap();
        
        assert_eq!(state.graph.selected_index(), 19);
    }

    // Helper pour créer un état de test
    fn create_test_state_with_graph(size: usize) -> AppState {
        // Créer un état minimal avec un graph de test
        // ...
    }
}
```

### 3.2 Tests du StagingHandler

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Setup un repo temporaire pour les tests.
    fn setup_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().unwrap();
        // Initialiser le repo, créer des fichiers, etc.
        // Voir tests/common/mod.rs pour l'implémentation existante
        todo!()
    }

    #[test]
    fn test_stage_file_moves_to_staged() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string());
        
        // Créer un fichier non stagé
        std::fs::write(dir.path().join("test.txt"), "content").unwrap();
        refresh_staging(&mut state).unwrap();
        
        assert_eq!(state.staging_state.unstaged.len(), 1);
        assert_eq!(state.staging_state.staged.len(), 0);
        
        // Stager le fichier
        let handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };
        handler.handle(&mut ctx, StagingAction::StageFile).unwrap();
        
        refresh_staging(&mut state).unwrap();
        
        assert_eq!(state.staging_state.unstaged.len(), 0);
        assert_eq!(state.staging_state.staged.len(), 1);
    }

    #[test]
    fn test_commit_clears_message_and_staged() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string());
        
        // Setup: créer et stager un fichier
        std::fs::write(dir.path().join("test.txt"), "content").unwrap();
        stage_all(&state.repo.repo).unwrap();
        
        state.staging_state.commit_message = "Test commit".to_string();
        
        let handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };
        handler.handle(&mut ctx, StagingAction::ConfirmCommit).unwrap();
        
        assert!(state.staging_state.commit_message.is_empty());
        assert_eq!(state.staging_state.staged.len(), 0);
    }

    #[test]
    fn test_commit_with_empty_message_fails() {
        let (dir, repo) = setup_test_repo();
        let mut state = AppState::new(repo, dir.path().to_string_lossy().to_string());
        
        state.staging_state.commit_message = "".to_string();
        
        let handler = StagingHandler;
        let mut ctx = HandlerContext { state: &mut state };
        handler.handle(&mut ctx, StagingAction::ConfirmCommit).unwrap();
        
        // Devrait avoir un message d'erreur
        assert!(state.flash_message.is_some());
        assert!(state.flash_message.as_ref().unwrap().text.contains("vide"));
    }
}
```

---

## 4. Tests des composants UI (Snapshot Testing)

### 4.1 Setup pour les tests UI

#### Ajout dans `Cargo.toml`

```toml
[dev-dependencies]
# ... existants ...
insta = { version = "1.34", features = ["yaml"] }
```

#### Fichier: `src/ui/tests/mod.rs`

```rust
//! Tests snapshot pour les composants UI.

use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::Rect,
    Terminal,
};

/// Helper pour capturer le rendu d'un composant.
pub fn render_to_string<F>(width: u16, height: u16, render_fn: F) -> String
where
    F: FnOnce(&mut ratatui::Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|frame| {
        render_fn(frame);
    }).unwrap();
    
    let buffer = terminal.backend().buffer();
    buffer_to_string(buffer)
}

fn buffer_to_string(buffer: &Buffer) -> String {
    let mut output = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.get(x, y);
            output.push_str(cell.symbol());
        }
        output.push('\n');
    }
    output
}
```

### 4.2 Tests snapshot pour le graph_view

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::graph::{GraphRow, CommitNode};
    use crate::ui::tests::render_to_string;
    use insta::assert_snapshot;

    fn create_test_graph() -> Vec<GraphRow> {
        vec![
            GraphRow {
                node: CommitNode {
                    oid: "abc1234".to_string(),
                    message: "First commit".to_string(),
                    author: "Alice".to_string(),
                    // ... autres champs
                },
                column: 0,
                // ...
            },
            GraphRow {
                node: CommitNode {
                    oid: "def5678".to_string(),
                    message: "Second commit".to_string(),
                    author: "Bob".to_string(),
                    // ...
                },
                column: 0,
                // ...
            },
        ]
    }

    #[test]
    fn test_graph_view_render() {
        let graph = create_test_graph();
        
        let output = render_to_string(80, 10, |frame| {
            let area = frame.size();
            render_graph_view(frame, area, &graph, 0, 0, true);
        });
        
        assert_snapshot!(output);
    }

    #[test]
    fn test_graph_view_with_selection() {
        let graph = create_test_graph();
        
        let output = render_to_string(80, 10, |frame| {
            let area = frame.size();
            render_graph_view(frame, area, &graph, 1, 0, true);  // Sélection sur le 2e item
        });
        
        assert_snapshot!(output);
    }
}
```

Les snapshots seront stockés dans `src/ui/tests/snapshots/`.

---

## 5. Tests d'intégration

### Fichier: `tests/integration/mod.rs`

```rust
//! Tests d'intégration pour les workflows complets.

mod common;

use common::TestRepo;

#[test]
fn test_full_commit_workflow() {
    // 1. Créer un repo de test
    let test_repo = TestRepo::new();
    
    // 2. Créer un fichier
    test_repo.create_file("test.txt", "Hello, World!");
    
    // 3. Simuler les actions: stage, commit
    let mut app = create_test_app(&test_repo);
    
    // Stage le fichier
    app.apply_action(AppAction::Staging(StagingAction::StageAll)).unwrap();
    
    // Vérifier qu'il est stagé
    assert_eq!(app.state.staging_state.staged.len(), 1);
    
    // Écrire un message et committer
    app.state.staging_state.commit_message = "Test commit".to_string();
    app.apply_action(AppAction::Staging(StagingAction::ConfirmCommit)).unwrap();
    
    // Vérifier le commit
    assert!(test_repo.last_commit_message().unwrap().contains("Test commit"));
}

#[test]
fn test_branch_create_and_checkout() {
    let test_repo = TestRepo::new();
    test_repo.initial_commit();
    
    let mut app = create_test_app(&test_repo);
    
    // Créer une nouvelle branche
    app.state.branches_view_state.input_value = "feature/test".to_string();
    app.apply_action(AppAction::Branch(BranchAction::Create)).unwrap();
    
    // Vérifier que la branche existe
    let branches = test_repo.list_branches();
    assert!(branches.contains(&"feature/test".to_string()));
    
    // Checkout
    app.state.branches_view_state.local_branches.select(1);  // Sélectionner feature/test
    app.apply_action(AppAction::Branch(BranchAction::Checkout)).unwrap();
    
    // Vérifier qu'on est sur la nouvelle branche
    assert_eq!(test_repo.current_branch(), "feature/test");
}

#[test]
fn test_stash_save_and_pop() {
    let test_repo = TestRepo::new();
    test_repo.initial_commit();
    test_repo.create_file("test.txt", "modified content");
    
    let mut app = create_test_app(&test_repo);
    
    // Vérifier qu'il y a des modifications
    assert!(!app.state.staging_state.unstaged.is_empty());
    
    // Stash
    app.apply_action(AppAction::Branch(BranchAction::StashSave)).unwrap();
    
    // Vérifier que le working directory est propre
    app.refresh_staging().unwrap();
    assert!(app.state.staging_state.unstaged.is_empty());
    
    // Pop stash
    app.state.branches_view_state.stash_selected = 0;
    app.apply_action(AppAction::Branch(BranchAction::StashPop)).unwrap();
    
    // Vérifier que les modifications sont revenues
    app.refresh_staging().unwrap();
    assert!(!app.state.staging_state.unstaged.is_empty());
}
```

---

## 6. Couverture de code

### Setup avec cargo-tarpaulin

```bash
# Installation
cargo install cargo-tarpaulin

# Exécution
cargo tarpaulin --out Html --output-dir coverage/

# Ouvrir le rapport
open coverage/tarpaulin-report.html
```

### Configuration dans `Cargo.toml`

```toml
[package.metadata.tarpaulin]
timeout = "300"
out = ["Html", "Json"]
output-dir = "coverage"
exclude-files = ["tests/*", "src/main.rs"]
```

---

## 7. Checklist des tests à implémenter

### Handlers (priorité haute)
- [ ] `NavigationHandler` - 8 tests
- [ ] `StagingHandler` - 10 tests
- [ ] `BranchHandler` - 8 tests
- [ ] `SearchHandler` - 5 tests
- [ ] `ConflictHandler` - 15 tests

### State (priorité moyenne)
- [ ] `ListSelection<T>` - 10 tests (partiellement fait dans STEP_04)
- [ ] `DiffCache` - 5 tests (fait dans STEP_06)
- [ ] `AppAction` dispatching - 5 tests

### UI (priorité basse)
- [ ] `graph_view` snapshots - 5 tests
- [ ] `staging_view` snapshots - 3 tests
- [ ] `branches_view` snapshots - 3 tests
- [ ] `confirm_dialog` snapshots - 2 tests

### Intégration (priorité moyenne)
- [ ] Workflow commit complet - 1 test
- [ ] Workflow branches - 1 test
- [ ] Workflow stash - 1 test
- [ ] Workflow merge avec conflits - 1 test

---

## 8. Commandes de validation

```bash
# Tests unitaires
cargo test

# Tests avec sortie verbose
cargo test -- --nocapture

# Tests d'un module spécifique
cargo test handler::navigation

# Mise à jour des snapshots
cargo insta review

# Couverture
cargo tarpaulin

# Tous les tests avec rapport
cargo test && cargo tarpaulin --out Html
```

---

## Objectifs de couverture

| Module | Couverture actuelle | Objectif |
|--------|-------------------|----------|
| `git/*` | ~60% | 80% |
| `handler/*` | 0% | 70% |
| `state/*` | 0% | 60% |
| `ui/*` | 0% | 40% |
| **Total** | ~30% | **60%** |
