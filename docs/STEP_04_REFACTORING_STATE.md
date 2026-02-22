# STEP 04 - Refactoring de State.rs

**Priorité**: 🔴 Haute  
**Effort estimé**: 4-6 heures  
**Risque**: Élevé (modification centrale)  
**Prérequis**: STEP_01, STEP_02, STEP_03 complétés

---

## Objectif

Restructurer le fichier `state.rs` (~600 lignes) qui contient 17 types et trop de responsabilités :

1. Extraire un type générique `ListSelection<T>` pour la gestion des listes
2. Diviser `AppAction` (100+ variants) en sous-enums par domaine
3. Organiser les view states dans des modules dédiés
4. Réduire le couplage de `AppState`

---

## 1. Structure cible

```
src/state/
├── mod.rs              # AppState (réduit) + re-exports
├── action/
│   ├── mod.rs          # AppAction (délégation)
│   ├── navigation.rs   # NavigationAction
│   ├── git.rs          # GitAction
│   ├── staging.rs      # StagingAction
│   ├── branch.rs       # BranchAction
│   ├── conflict.rs     # ConflictAction
│   ├── search.rs       # SearchAction
│   └── edit.rs         # EditAction
├── view/
│   ├── mod.rs          # ViewMode + re-exports
│   ├── graph.rs        # GraphViewState (nouveau)
│   ├── staging.rs      # StagingState + StagingFocus
│   ├── branches.rs     # BranchesViewState + enums
│   ├── blame.rs        # BlameState
│   ├── conflicts.rs    # ConflictsState + ConflictPanelFocus
│   ├── search.rs       # SearchState
│   └── merge_picker.rs # MergePickerState
├── selection.rs        # ListSelection<T>
└── cache.rs            # DiffCache
```

---

## 2. Type générique `ListSelection<T>`

### Fichier: `src/state/selection.rs`

```rust
//! Gestion générique de sélection dans une liste avec scroll.

use std::ops::{Deref, DerefMut};

/// Gère la sélection et le scroll dans une liste d'éléments.
#[derive(Debug, Clone, Default)]
pub struct ListSelection<T> {
    items: Vec<T>,
    selected: usize,
    scroll_offset: usize,
    visible_height: usize,
}

impl<T> ListSelection<T> {
    /// Crée une nouvelle sélection vide.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            visible_height: 10, // Valeur par défaut
        }
    }

    /// Crée une sélection avec des éléments.
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            items,
            selected: 0,
            scroll_offset: 0,
            visible_height: 10,
        }
    }

    /// Définit la hauteur visible (pour le scroll).
    pub fn set_visible_height(&mut self, height: usize) {
        self.visible_height = height;
        self.adjust_scroll();
    }

    /// Remplace les éléments.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        // Ajuster la sélection si nécessaire
        if self.selected >= self.items.len() && !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
        self.adjust_scroll();
    }

    /// Index de l'élément sélectionné.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Élément actuellement sélectionné.
    pub fn selected_item(&self) -> Option<&T> {
        self.items.get(self.selected)
    }

    /// Élément actuellement sélectionné (mutable).
    pub fn selected_item_mut(&mut self) -> Option<&mut T> {
        self.items.get_mut(self.selected)
    }

    /// Offset de scroll actuel.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Nombre d'éléments.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// La liste est-elle vide?
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Sélectionne l'élément précédent.
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.adjust_scroll();
        }
    }

    /// Sélectionne l'élément suivant.
    pub fn select_next(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
            self.adjust_scroll();
        }
    }

    /// Remonte d'une page.
    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(self.visible_height);
        self.adjust_scroll();
    }

    /// Descend d'une page.
    pub fn page_down(&mut self) {
        self.selected = (self.selected + self.visible_height).min(
            self.items.len().saturating_sub(1)
        );
        self.adjust_scroll();
    }

    /// Va au premier élément.
    pub fn select_first(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Va au dernier élément.
    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.selected = self.items.len() - 1;
            self.adjust_scroll();
        }
    }

    /// Sélectionne un index spécifique.
    pub fn select(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected = index;
            self.adjust_scroll();
        }
    }

    /// Ajuste le scroll pour garder la sélection visible.
    fn adjust_scroll(&mut self) {
        // La sélection est au-dessus de la zone visible
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
        // La sélection est en-dessous de la zone visible
        if self.selected >= self.scroll_offset + self.visible_height {
            self.scroll_offset = self.selected - self.visible_height + 1;
        }
    }

    /// Itère sur les éléments visibles avec leur index original.
    pub fn visible_items(&self) -> impl Iterator<Item = (usize, &T)> {
        self.items
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.visible_height)
    }
}

impl<T> Deref for ListSelection<T> {
    type Target = Vec<T>;
    
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T> DerefMut for ListSelection<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_next() {
        let mut sel = ListSelection::with_items(vec![1, 2, 3, 4, 5]);
        assert_eq!(sel.selected_index(), 0);
        
        sel.select_next();
        assert_eq!(sel.selected_index(), 1);
        
        sel.select_next();
        sel.select_next();
        sel.select_next();
        assert_eq!(sel.selected_index(), 4);
        
        // Ne dépasse pas la fin
        sel.select_next();
        assert_eq!(sel.selected_index(), 4);
    }

    #[test]
    fn test_scroll_adjustment() {
        let mut sel = ListSelection::with_items(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        sel.set_visible_height(3);
        
        assert_eq!(sel.scroll_offset(), 0);
        
        sel.select(5);
        assert!(sel.scroll_offset() > 0);
    }

    #[test]
    fn test_empty_list() {
        let mut sel: ListSelection<i32> = ListSelection::new();
        sel.select_next();
        sel.select_previous();
        assert_eq!(sel.selected_index(), 0);
        assert!(sel.selected_item().is_none());
    }
}
```

---

## 3. Division de `AppAction`

### Fichier: `src/state/action/mod.rs`

```rust
//! Actions de l'application organisées par domaine.

mod navigation;
mod git;
mod staging;
mod branch;
mod conflict;
mod search;
mod edit;

pub use navigation::NavigationAction;
pub use git::GitAction;
pub use staging::StagingAction;
pub use branch::BranchAction;
pub use conflict::ConflictAction;
pub use search::SearchAction;
pub use edit::EditAction;

/// Action principale de l'application.
/// 
/// Délègue vers des sous-enums spécialisés pour une meilleure organisation.
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    /// Quitter l'application
    Quit,
    
    /// Rafraîchir les données
    Refresh,
    
    /// Actions de navigation
    Navigation(NavigationAction),
    
    /// Actions git (push, pull, fetch, etc.)
    Git(GitAction),
    
    /// Actions de staging/commit
    Staging(StagingAction),
    
    /// Actions sur les branches
    Branch(BranchAction),
    
    /// Actions de résolution de conflits
    Conflict(ConflictAction),
    
    /// Actions de recherche
    Search(SearchAction),
    
    /// Actions d'édition de texte
    Edit(EditAction),
    
    /// Changer de mode de vue
    SwitchView(ViewMode),
    
    /// Afficher/masquer l'aide
    ToggleHelp,
    
    /// Copier dans le presse-papier
    CopyToClipboard,
    
    /// Aucune action (événement ignoré)
    None,
}

use super::view::ViewMode;
```

### Fichier: `src/state/action/navigation.rs`

```rust
//! Actions de navigation dans les listes et panneaux.

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationAction {
    /// Monter d'un élément
    MoveUp,
    /// Descendre d'un élément
    MoveDown,
    /// Remonter d'une page
    PageUp,
    /// Descendre d'une page
    PageDown,
    /// Aller au premier élément
    GoTop,
    /// Aller au dernier élément
    GoBottom,
    /// Changer de panneau (Tab)
    SwitchPanel,
    /// Faire défiler le diff vers le haut
    ScrollDiffUp,
    /// Faire défiler le diff vers le bas
    ScrollDiffDown,
}
```

### Fichier: `src/state/action/git.rs`

```rust
//! Actions git (opérations remote, etc.)

#[derive(Debug, Clone, PartialEq)]
pub enum GitAction {
    /// Push vers le remote
    Push,
    /// Pull depuis le remote
    Pull,
    /// Fetch depuis le remote
    Fetch,
    /// Cherry-pick un commit
    CherryPick,
    /// Amender le dernier commit
    AmendCommit,
    /// Ouvrir le blame d'un fichier
    OpenBlame,
    /// Fermer le blame
    CloseBlame,
    /// Aller au commit du blame
    JumpToBlameCommit,
}
```

### Fichier: `src/state/action/staging.rs`

```rust
//! Actions de staging et commit.

#[derive(Debug, Clone, PartialEq)]
pub enum StagingAction {
    /// Ajouter un fichier au staging
    StageFile,
    /// Retirer un fichier du staging
    UnstageFile,
    /// Ajouter tous les fichiers
    StageAll,
    /// Retirer tous les fichiers
    UnstageAll,
    /// Commencer l'édition du message de commit
    StartCommitMessage,
    /// Valider le commit
    ConfirmCommit,
    /// Annuler le commit
    CancelCommit,
    /// Discard les modifications d'un fichier
    DiscardFile,
    /// Discard toutes les modifications
    DiscardAll,
}
```

### Fichier: `src/state/action/branch.rs`

```rust
//! Actions sur les branches, worktrees et stashes.

#[derive(Debug, Clone, PartialEq)]
pub enum BranchAction {
    /// Lister les branches
    List,
    /// Checkout une branche
    Checkout,
    /// Créer une branche
    Create,
    /// Supprimer une branche
    Delete,
    /// Renommer une branche
    Rename,
    /// Afficher/masquer les branches distantes
    ToggleRemote,
    /// Merger une branche
    Merge,
    /// Créer un stash
    StashSave,
    /// Appliquer un stash
    StashApply,
    /// Pop un stash
    StashPop,
    /// Supprimer un stash
    StashDrop,
    /// Créer un worktree
    WorktreeCreate,
    /// Supprimer un worktree
    WorktreeRemove,
}
```

### Fichier: `src/state/action/conflict.rs`

```rust
//! Actions de résolution de conflits.

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictAction {
    /// Naviguer vers le fichier précédent
    PreviousFile,
    /// Naviguer vers le fichier suivant
    NextFile,
    /// Naviguer vers la section précédente
    PreviousSection,
    /// Naviguer vers la section suivante
    NextSection,
    /// Changer de panneau
    SwitchPanel,
    /// Accepter notre version (fichier entier)
    AcceptOursFile,
    /// Accepter leur version (fichier entier)
    AcceptTheirsFile,
    /// Accepter notre version (bloc)
    AcceptOursBlock,
    /// Accepter leur version (bloc)
    AcceptTheirsBlock,
    /// Accepter les deux versions
    AcceptBoth,
    /// Activer le mode édition
    StartEdit,
    /// Valider l'édition
    ConfirmEdit,
    /// Annuler l'édition
    CancelEdit,
    /// Marquer le fichier comme résolu
    MarkResolved,
    /// Finaliser le merge
    FinalizeMerge,
    /// Abandonner le merge
    AbortMerge,
}
```

### Fichier: `src/state/action/search.rs`

```rust
//! Actions de recherche.

#[derive(Debug, Clone, PartialEq)]
pub enum SearchAction {
    /// Ouvrir la recherche
    Open,
    /// Fermer la recherche
    Close,
    /// Résultat suivant
    NextResult,
    /// Résultat précédent
    PreviousResult,
    /// Changer le type de recherche
    ChangeType,
    /// Exécuter la recherche
    Execute,
}
```

### Fichier: `src/state/action/edit.rs`

```rust
//! Actions d'édition de texte.

#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    /// Insérer un caractère
    InsertChar(char),
    /// Supprimer le caractère avant le curseur
    DeleteCharBefore,
    /// Supprimer le caractère après le curseur
    DeleteCharAfter,
    /// Déplacer le curseur à gauche
    CursorLeft,
    /// Déplacer le curseur à droite
    CursorRight,
    /// Aller au début de la ligne
    CursorHome,
    /// Aller à la fin de la ligne
    CursorEnd,
    /// Nouvelle ligne
    NewLine,
}
```

---

## 4. View States dans des modules dédiés

### Fichier: `src/state/view/mod.rs`

```rust
//! États spécifiques à chaque vue.

mod graph;
mod staging;
mod branches;
mod blame;
mod conflicts;
mod search;
mod merge_picker;

pub use graph::GraphViewState;
pub use staging::{StagingState, StagingFocus};
pub use branches::{BranchesViewState, BranchesSection, BranchesFocus};
pub use blame::BlameState;
pub use conflicts::{ConflictsState, ConflictPanelFocus};
pub use search::SearchState;
pub use merge_picker::MergePickerState;

/// Mode de vue actif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Graph,
    Staging,
    Branches,
    Conflicts,
    Blame,
    Help,
}

/// Mode d'affichage du panneau bottom-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BottomLeftMode {
    #[default]
    Files,
    Parents,
}

/// Panneau ayant le focus dans la vue principale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusPanel {
    #[default]
    Graph,
    BottomLeft,
    BottomRight,
}
```

### Fichier: `src/state/view/staging.rs`

```rust
//! État de la vue staging.

use crate::git::repo::StatusEntry;
use crate::state::selection::ListSelection;

/// Focus dans la vue staging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StagingFocus {
    #[default]
    Unstaged,
    Staged,
    Diff,
    CommitMessage,
}

/// État complet de la vue staging.
#[derive(Debug, Clone, Default)]
pub struct StagingState {
    /// Fichiers non stagés.
    pub unstaged: ListSelection<StatusEntry>,
    /// Fichiers stagés.
    pub staged: ListSelection<StatusEntry>,
    /// Panneau actif.
    pub focus: StagingFocus,
    /// Message de commit en cours.
    pub commit_message: String,
    /// Position du curseur dans le message.
    pub cursor_position: usize,
    /// Diff du fichier sélectionné.
    pub current_diff: Option<String>,
    /// Offset de scroll du diff.
    pub diff_scroll: usize,
}

impl StagingState {
    /// Crée un nouvel état staging.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fichier actuellement sélectionné (unstaged ou staged selon focus).
    pub fn selected_file(&self) -> Option<&StatusEntry> {
        match self.focus {
            StagingFocus::Unstaged => self.unstaged.selected_item(),
            StagingFocus::Staged => self.staged.selected_item(),
            _ => None,
        }
    }

    /// Passe au panneau suivant.
    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            StagingFocus::Unstaged => StagingFocus::Staged,
            StagingFocus::Staged => StagingFocus::Diff,
            StagingFocus::Diff => StagingFocus::Unstaged,
            StagingFocus::CommitMessage => StagingFocus::CommitMessage,
        };
    }
}
```

---

## 5. `AppState` allégé

### Fichier: `src/state/mod.rs`

```rust
//! État global de l'application.

pub mod action;
pub mod view;
pub mod selection;
pub mod cache;

pub use action::AppAction;
pub use view::*;
pub use selection::ListSelection;
pub use cache::DiffCache;

use crate::git::repo::GitRepo;
use crate::git::graph::GraphRow;

/// État global de l'application.
pub struct AppState {
    // ═══════════════════════════════════════════════════
    // Core
    // ═══════════════════════════════════════════════════
    
    /// Repository git.
    pub repo: GitRepo,
    
    /// Chemin du repository.
    pub repo_path: String,
    
    /// Branche courante.
    pub current_branch: Option<String>,
    
    // ═══════════════════════════════════════════════════
    // Vue active
    // ═══════════════════════════════════════════════════
    
    /// Mode de vue actuel.
    pub view_mode: ViewMode,
    
    /// État indiquant si un refresh est nécessaire.
    dirty: bool,
    
    // ═══════════════════════════════════════════════════
    // Vue Graph (toujours chargée)
    // ═══════════════════════════════════════════════════
    
    /// Lignes du graph de commits.
    pub graph: ListSelection<GraphRow>,
    
    /// Mode d'affichage du panneau bottom-left.
    pub bottom_left_mode: BottomLeftMode,
    
    /// Panneau avec focus.
    pub focus_panel: FocusPanel,
    
    // ═══════════════════════════════════════════════════
    // Vues optionnelles (chargées à la demande)
    // ═══════════════════════════════════════════════════
    
    /// État de la vue staging.
    pub staging_state: StagingState,
    
    /// État de la vue branches.
    pub branches_view_state: BranchesViewState,
    
    /// État du blame (si actif).
    pub blame_state: Option<BlameState>,
    
    /// État de résolution de conflits (si actif).
    pub conflicts_state: Option<ConflictsState>,
    
    /// État de la recherche.
    pub search_state: SearchState,
    
    /// Picker de merge (si actif).
    pub merge_picker: Option<MergePickerState>,
    
    // ═══════════════════════════════════════════════════
    // UI transient
    // ═══════════════════════════════════════════════════
    
    /// Message flash à afficher.
    pub flash_message: Option<FlashMessage>,
    
    /// Confirmation en attente.
    pub pending_confirm: Option<ConfirmAction>,
    
    /// Spinner de chargement.
    pub loading: Option<LoadingSpinner>,
    
    // ═══════════════════════════════════════════════════
    // Cache
    // ═══════════════════════════════════════════════════
    
    /// Cache des diffs.
    diff_cache: DiffCache,
}

impl AppState {
    /// Crée un nouvel état d'application.
    pub fn new(repo: GitRepo, repo_path: String) -> Self {
        let current_branch = repo.current_branch().ok();
        
        Self {
            repo,
            repo_path,
            current_branch,
            view_mode: ViewMode::Graph,
            dirty: true,
            graph: ListSelection::new(),
            bottom_left_mode: BottomLeftMode::Files,
            focus_panel: FocusPanel::Graph,
            staging_state: StagingState::new(),
            branches_view_state: BranchesViewState::new(),
            blame_state: None,
            conflicts_state: None,
            search_state: SearchState::default(),
            merge_picker: None,
            flash_message: None,
            pending_confirm: None,
            loading: None,
            diff_cache: DiffCache::new(50),
        }
    }

    /// Marque l'état comme nécessitant un refresh.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// L'état nécessite-t-il un refresh?
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marque l'état comme propre.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Définit un message flash.
    pub fn set_flash_message(&mut self, message: impl Into<String>) {
        self.flash_message = Some(FlashMessage::new(message.into()));
    }

    /// Efface le message flash.
    pub fn clear_flash_message(&mut self) {
        self.flash_message = None;
    }

    /// Accès au cache de diff.
    pub fn diff_cache(&mut self) -> &mut DiffCache {
        &mut self.diff_cache
    }
}

/// Message flash temporaire.
#[derive(Debug, Clone)]
pub struct FlashMessage {
    pub text: String,
    pub created_at: std::time::Instant,
}

impl FlashMessage {
    pub fn new(text: String) -> Self {
        Self {
            text,
            created_at: std::time::Instant::now(),
        }
    }

    /// Le message a-t-il expiré?
    pub fn is_expired(&self, duration_secs: u64) -> bool {
        self.created_at.elapsed().as_secs() >= duration_secs
    }
}

/// Action de confirmation en attente.
#[derive(Debug, Clone)]
pub struct ConfirmAction {
    pub message: String,
    pub action_type: ConfirmActionType,
}

#[derive(Debug, Clone)]
pub enum ConfirmActionType {
    DeleteBranch(String),
    DiscardFile(String),
    DiscardAll,
    DropStash(usize),
    AbortMerge,
}

/// Spinner de chargement.
#[derive(Debug, Clone)]
pub struct LoadingSpinner {
    pub message: String,
    pub frame: usize,
}
```

---

## 6. Plan de migration

### Étape 1: Créer la structure de fichiers
```bash
mkdir -p src/state/action
mkdir -p src/state/view
touch src/state/mod.rs
touch src/state/selection.rs
touch src/state/cache.rs
touch src/state/action/mod.rs
touch src/state/action/navigation.rs
# ... etc
```

### Étape 2: Migrer `ListSelection<T>`
1. Créer `src/state/selection.rs`
2. Ajouter les tests
3. Compiler et vérifier

### Étape 3: Migrer les actions
1. Créer chaque fichier d'action
2. Modifier `AppAction` pour déléguer
3. Mettre à jour `src/ui/input.rs` pour retourner les nouvelles actions

### Étape 4: Migrer les view states
1. Créer chaque fichier de view state
2. Mettre à jour `AppState` pour utiliser les nouveaux types
3. Mettre à jour les références dans `event.rs`

### Étape 5: Adapter les handlers
Les handlers dans `event.rs` devront être adaptés pour le pattern matching imbriqué:

```rust
// AVANT
match action {
    AppAction::MoveUp => self.handle_move_up()?,
    AppAction::MoveDown => self.handle_move_down()?,
    // ...100 autres cas
}

// APRÈS
match action {
    AppAction::Navigation(nav) => self.handle_navigation(nav)?,
    AppAction::Git(git) => self.handle_git(git)?,
    AppAction::Staging(staging) => self.handle_staging(staging)?,
    AppAction::Quit => self.should_quit = true,
    // ...
}
```

---

## 7. Checklist de validation

```bash
# 1. Créer tous les fichiers
tree src/state/

# 2. Compiler progressivement
cargo check

# 3. Tests
cargo test

# 4. Vérifier que l'ancien state.rs est vide ou supprimé
wc -l src/state.rs  # Devrait être 0 ou le fichier supprimé

# 5. Clippy
cargo clippy --all-features -- -D warnings

# 6. Test manuel
cargo run
```

---

## Bénéfices attendus

| Métrique | Avant | Après |
|----------|-------|-------|
| Lignes dans state.rs | 600+ | ~150 (mod.rs) |
| Variants dans AppAction | 100+ | ~15 (délégation) |
| Duplication de logique sélection | ~15x | 0 (ListSelection) |
| Couplage entre vues | Élevé | Faible |
| Testabilité | Difficile | Facile |
