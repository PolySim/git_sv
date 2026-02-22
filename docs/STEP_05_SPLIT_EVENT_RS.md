# STEP 05 - Split de event.rs en Modules Handlers

**Priorité**: 🔴 Haute  
**Effort estimé**: 6-8 heures  
**Risque**: Élevé (fichier central de ~3400 lignes)  
**Prérequis**: STEP_01 à STEP_04 complétés

---

## Objectif

Le fichier `event.rs` fait ~3400 lignes avec 98 handlers et devient impossible à maintenir. L'objectif est de le découper en modules spécialisés tout en gardant une interface unifiée.

---

## 1. Analyse du fichier actuel

### Structure actuelle

```
event.rs (3400 lignes)
├── Imports (1-20)
├── copy_to_clipboard() fonction libre (1-20)
├── EventHandler struct (22-28)
├── impl EventHandler
│   ├── run() - boucle principale (30-60)
│   ├── apply_action() - dispatcher (~130 lignes)
│   ├── handle_* methods (~3200 lignes)
│   │   ├── Navigation (326-441)
│   │   ├── View switching (443-497)
│   │   ├── Branch operations (499-548, 840-916, 1226-1250)
│   │   ├── File navigation (550-614)
│   │   ├── Staging (616-745)
│   │   ├── Input handling (749-837)
│   │   ├── Stash (996-1118)
│   │   ├── Worktree (962-994)
│   │   ├── Remote operations (1658-1772)
│   │   ├── Search (1774-1850)
│   │   ├── Blame (1933-2052)
│   │   ├── Cherry-pick & Amend (2056-2150)
│   │   ├── Merge (2154-2280)
│   │   ├── Conflict resolution (2285-3220) ← 935 lignes!
│   │   └── Clipboard (3226-3403)
│   └── refresh() et helpers
```

### Problèmes identifiés

1. **Taille**: Impossible de naviguer efficacement
2. **Couplage**: Tous les handlers accèdent à `self.state` directement
3. **Duplication**: Patterns répétés (flash messages, refresh)
4. **Test**: Impossible de tester les handlers individuellement

---

## 2. Structure cible

```
src/handler/
├── mod.rs              # EventHandler + run() + apply_action()
├── traits.rs           # ActionHandler trait
├── context.rs          # HandlerContext (accès limité à l'état)
├── helpers.rs          # flash_message, refresh, etc.
├── navigation.rs       # NavigationHandler
├── view.rs             # ViewHandler (switch views)
├── staging.rs          # StagingHandler
├── branch.rs           # BranchHandler
├── stash.rs            # StashHandler
├── worktree.rs         # WorktreeHandler
├── remote.rs           # RemoteHandler (push/pull/fetch)
├── search.rs           # SearchHandler
├── blame.rs            # BlameHandler
├── merge.rs            # MergeHandler
├── clipboard.rs        # ClipboardHandler
├── edit.rs             # EditHandler (input text)
└── conflict/
    ├── mod.rs          # ConflictHandler entry point
    ├── navigation.rs   # File/section/line navigation
    ├── resolution.rs   # Accept ours/theirs/both
    ├── editing.rs      # Line editing mode
    └── finalize.rs     # Commit/abort merge
```

---

## 3. Trait `ActionHandler`

### Fichier: `src/handler/traits.rs`

```rust
//! Traits pour les handlers d'actions.

use crate::error::Result;
use crate::state::AppState;

/// Contexte minimal pour les handlers.
pub struct HandlerContext<'a> {
    pub state: &'a mut AppState,
}

/// Trait pour les handlers spécialisés.
pub trait ActionHandler {
    /// Type d'action géré par ce handler.
    type Action;

    /// Peut-on gérer cette action dans l'état actuel?
    fn can_handle(&self, state: &AppState, action: &Self::Action) -> bool {
        let _ = (state, action);
        true
    }

    /// Exécute l'action.
    fn handle(&mut self, ctx: &mut HandlerContext, action: Self::Action) -> Result<()>;
}

/// Extension pour les résultats avec message flash.
pub trait ResultExt<T> {
    fn with_flash(self, state: &mut AppState, success_msg: &str) -> Result<T>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for std::result::Result<T, E> {
    fn with_flash(self, state: &mut AppState, success_msg: &str) -> Result<T> {
        match self {
            Ok(val) => {
                state.set_flash_message(format!("{} ✓", success_msg));
                Ok(val)
            }
            Err(e) => {
                state.set_flash_message(format!("❌ {}", e));
                Err(crate::error::GitSvError::OperationFailed {
                    operation: success_msg,
                    details: e.to_string(),
                })
            }
        }
    }
}
```

---

## 4. Helpers communs

### Fichier: `src/handler/helpers.rs`

```rust
//! Fonctions utilitaires partagées entre handlers.

use crate::error::Result;
use crate::state::AppState;
use crate::git::graph::GraphRow;

/// Nombre max de commits à charger.
pub const MAX_COMMITS: usize = 500;

/// Rafraîchit le graph de commits.
pub fn refresh_graph(state: &mut AppState) -> Result<()> {
    let graph = state.repo.build_graph(MAX_COMMITS)?;
    state.graph.set_items(graph);
    state.mark_dirty();
    Ok(())
}

/// Rafraîchit l'état de staging.
pub fn refresh_staging(state: &mut AppState) -> Result<()> {
    let unstaged = state.repo.status_unstaged()?;
    let staged = state.repo.status_staged()?;
    
    state.staging_state.unstaged.set_items(unstaged);
    state.staging_state.staged.set_items(staged);
    
    // Invalider le diff cache pour les fichiers working directory
    state.diff_cache().clear_working_directory();
    
    Ok(())
}

/// Rafraîchit les branches.
pub fn refresh_branches(state: &mut AppState) -> Result<()> {
    let branches = state.repo.list_branches()?;
    state.branches_view_state.local_branches.set_items(branches);
    
    if state.branches_view_state.show_remote {
        let remote = state.repo.list_remote_branches()?;
        state.branches_view_state.remote_branches.set_items(remote);
    }
    
    Ok(())
}

/// Rafraîchit toutes les données.
pub fn refresh_all(state: &mut AppState) -> Result<()> {
    refresh_graph(state)?;
    refresh_staging(state)?;
    refresh_branches(state)?;
    state.current_branch = state.repo.current_branch().ok();
    Ok(())
}

/// Copie du texte dans le presse-papier.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    use arboard::Clipboard;
    
    let mut clipboard = Clipboard::new()
        .map_err(|e| crate::error::GitSvError::Clipboard(e.to_string()))?;
    
    clipboard.set_text(text)
        .map_err(|e| crate::error::GitSvError::Clipboard(e.to_string()))?;
    
    Ok(())
}
```

---

## 5. Handler de navigation

### Fichier: `src/handler/navigation.rs`

```rust
//! Handler pour les actions de navigation.

use crate::error::Result;
use crate::state::{AppState, ViewMode, FocusPanel};
use crate::state::action::NavigationAction;
use super::traits::{ActionHandler, HandlerContext};

/// Handler pour la navigation dans les listes.
pub struct NavigationHandler;

impl ActionHandler for NavigationHandler {
    type Action = NavigationAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: NavigationAction) -> Result<()> {
        match action {
            NavigationAction::MoveUp => self.handle_move_up(ctx.state),
            NavigationAction::MoveDown => self.handle_move_down(ctx.state),
            NavigationAction::PageUp => self.handle_page_up(ctx.state),
            NavigationAction::PageDown => self.handle_page_down(ctx.state),
            NavigationAction::GoTop => self.handle_go_top(ctx.state),
            NavigationAction::GoBottom => self.handle_go_bottom(ctx.state),
            NavigationAction::SwitchPanel => self.handle_switch_panel(ctx.state),
            NavigationAction::ScrollDiffUp => self.handle_scroll_diff_up(ctx.state),
            NavigationAction::ScrollDiffDown => self.handle_scroll_diff_down(ctx.state),
        }
        Ok(())
    }
}

impl NavigationHandler {
    fn handle_move_up(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Graph => {
                match state.focus_panel {
                    FocusPanel::Graph => state.graph.select_previous(),
                    FocusPanel::BottomLeft => {
                        // Navigation dans les fichiers du commit
                    }
                    FocusPanel::BottomRight => {
                        // Scroll du diff
                    }
                }
            }
            ViewMode::Staging => {
                state.staging_state.navigate_up();
            }
            ViewMode::Branches => {
                state.branches_view_state.navigate_up();
            }
            // ... autres modes
            _ => {}
        }
    }

    fn handle_move_down(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Graph => {
                match state.focus_panel {
                    FocusPanel::Graph => state.graph.select_next(),
                    FocusPanel::BottomLeft => {}
                    FocusPanel::BottomRight => {}
                }
            }
            ViewMode::Staging => {
                state.staging_state.navigate_down();
            }
            ViewMode::Branches => {
                state.branches_view_state.navigate_down();
            }
            _ => {}
        }
    }

    fn handle_page_up(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Graph => state.graph.page_up(),
            ViewMode::Staging => state.staging_state.page_up(),
            _ => {}
        }
    }

    fn handle_page_down(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Graph => state.graph.page_down(),
            ViewMode::Staging => state.staging_state.page_down(),
            _ => {}
        }
    }

    fn handle_go_top(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Graph => state.graph.select_first(),
            ViewMode::Staging => state.staging_state.go_top(),
            _ => {}
        }
    }

    fn handle_go_bottom(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Graph => state.graph.select_last(),
            ViewMode::Staging => state.staging_state.go_bottom(),
            _ => {}
        }
    }

    fn handle_switch_panel(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Graph => {
                state.focus_panel = match state.focus_panel {
                    FocusPanel::Graph => FocusPanel::BottomLeft,
                    FocusPanel::BottomLeft => FocusPanel::BottomRight,
                    FocusPanel::BottomRight => FocusPanel::Graph,
                };
            }
            ViewMode::Staging => {
                state.staging_state.cycle_focus();
            }
            _ => {}
        }
    }

    fn handle_scroll_diff_up(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Staging => {
                state.staging_state.diff_scroll = 
                    state.staging_state.diff_scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn handle_scroll_diff_down(&self, state: &mut AppState) {
        match state.view_mode {
            ViewMode::Staging => {
                state.staging_state.diff_scroll += 1;
            }
            _ => {}
        }
    }
}
```

---

## 6. Handler de staging

### Fichier: `src/handler/staging.rs`

```rust
//! Handler pour les opérations de staging.

use crate::error::Result;
use crate::state::AppState;
use crate::state::action::StagingAction;
use crate::git::commit::{stage_file, unstage_file, stage_all, unstage_all, create_commit};
use super::traits::{ActionHandler, HandlerContext, ResultExt};
use super::helpers::refresh_staging;

/// Handler pour les opérations de staging/commit.
pub struct StagingHandler;

impl ActionHandler for StagingHandler {
    type Action = StagingAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: StagingAction) -> Result<()> {
        let state = ctx.state;
        
        match action {
            StagingAction::StageFile => {
                if let Some(entry) = state.staging_state.unstaged.selected_item() {
                    let path = entry.path.clone();
                    stage_file(&state.repo.repo, &path)
                        .with_flash(state, &format!("Staged: {}", path))?;
                    refresh_staging(state)?;
                }
            }
            
            StagingAction::UnstageFile => {
                if let Some(entry) = state.staging_state.staged.selected_item() {
                    let path = entry.path.clone();
                    unstage_file(&state.repo.repo, &path)
                        .with_flash(state, &format!("Unstaged: {}", path))?;
                    refresh_staging(state)?;
                }
            }
            
            StagingAction::StageAll => {
                stage_all(&state.repo.repo)
                    .with_flash(state, "Tous les fichiers stagés")?;
                refresh_staging(state)?;
            }
            
            StagingAction::UnstageAll => {
                unstage_all(&state.repo.repo)
                    .with_flash(state, "Tous les fichiers unstagés")?;
                refresh_staging(state)?;
            }
            
            StagingAction::StartCommitMessage => {
                state.staging_state.focus = crate::state::view::StagingFocus::CommitMessage;
            }
            
            StagingAction::ConfirmCommit => {
                let message = state.staging_state.commit_message.trim();
                if message.is_empty() {
                    state.set_flash_message("❌ Message de commit vide");
                    return Ok(());
                }
                
                create_commit(&state.repo.repo, message)
                    .with_flash(state, "Commit créé")?;
                
                state.staging_state.commit_message.clear();
                state.staging_state.cursor_position = 0;
                state.staging_state.focus = crate::state::view::StagingFocus::Unstaged;
                
                refresh_staging(state)?;
                super::helpers::refresh_graph(state)?;
            }
            
            StagingAction::CancelCommit => {
                state.staging_state.commit_message.clear();
                state.staging_state.cursor_position = 0;
                state.staging_state.focus = crate::state::view::StagingFocus::Unstaged;
            }
            
            StagingAction::DiscardFile => {
                if let Some(entry) = state.staging_state.unstaged.selected_item() {
                    let path = entry.path.clone();
                    // Demander confirmation avant de discard
                    state.pending_confirm = Some(crate::state::ConfirmAction {
                        message: format!("Abandonner les modifications de {} ?", path),
                        action_type: crate::state::ConfirmActionType::DiscardFile(path),
                    });
                }
            }
            
            StagingAction::DiscardAll => {
                state.pending_confirm = Some(crate::state::ConfirmAction {
                    message: "Abandonner TOUTES les modifications ?".into(),
                    action_type: crate::state::ConfirmActionType::DiscardAll,
                });
            }
        }
        
        Ok(())
    }
}
```

---

## 7. Handler principal (mod.rs)

### Fichier: `src/handler/mod.rs`

```rust
//! Gestionnaires d'événements et d'actions.

mod traits;
mod helpers;
mod navigation;
mod view;
mod staging;
mod branch;
mod stash;
mod worktree;
mod remote;
mod search;
mod blame;
mod merge;
mod clipboard;
mod edit;
mod conflict;

use crate::error::Result;
use crate::state::{AppState, AppAction};
use traits::HandlerContext;

// Re-exports
pub use helpers::refresh_all;

/// Gestionnaire principal des événements.
pub struct EventHandler {
    state: AppState,
    should_quit: bool,
    
    // Handlers spécialisés
    navigation: navigation::NavigationHandler,
    staging: staging::StagingHandler,
    branch: branch::BranchHandler,
    remote: remote::RemoteHandler,
    search: search::SearchHandler,
    blame: blame::BlameHandler,
    merge: merge::MergeHandler,
    conflict: conflict::ConflictHandler,
    clipboard: clipboard::ClipboardHandler,
    edit: edit::EditHandler,
}

impl EventHandler {
    /// Crée un nouveau gestionnaire d'événements.
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            should_quit: false,
            navigation: navigation::NavigationHandler,
            staging: staging::StagingHandler,
            branch: branch::BranchHandler,
            remote: remote::RemoteHandler,
            search: search::SearchHandler,
            blame: blame::BlameHandler,
            merge: merge::MergeHandler,
            conflict: conflict::ConflictHandler::new(),
            clipboard: clipboard::ClipboardHandler,
            edit: edit::EditHandler,
        }
    }

    /// Boucle principale.
    pub fn run(&mut self, terminal: &mut crate::terminal::Terminal) -> Result<()> {
        while !self.should_quit {
            // Refresh si nécessaire
            if self.state.is_dirty() {
                helpers::refresh_all(&mut self.state)?;
                self.state.mark_clean();
            }

            // Render
            terminal.draw(|frame| {
                crate::ui::render(frame, &self.state);
            })?;

            // Handle input
            if let Some(action) = crate::ui::input::handle_events(&self.state)? {
                self.apply_action(action)?;
            }

            // Clear expired flash message
            if let Some(ref flash) = self.state.flash_message {
                if flash.is_expired(3) {
                    self.state.clear_flash_message();
                }
            }
        }
        
        Ok(())
    }

    /// Dispatch une action vers le handler approprié.
    fn apply_action(&mut self, action: AppAction) -> Result<()> {
        use crate::state::action::*;
        use traits::ActionHandler;

        // Créer le contexte
        let mut ctx = HandlerContext {
            state: &mut self.state,
        };

        match action {
            AppAction::Quit => {
                self.should_quit = true;
            }
            
            AppAction::Refresh => {
                self.state.mark_dirty();
            }
            
            AppAction::Navigation(nav) => {
                self.navigation.handle(&mut ctx, nav)?;
            }
            
            AppAction::Staging(staging) => {
                self.staging.handle(&mut ctx, staging)?;
            }
            
            AppAction::Branch(branch) => {
                self.branch.handle(&mut ctx, branch)?;
            }
            
            AppAction::Git(git) => {
                self.remote.handle(&mut ctx, git)?;
            }
            
            AppAction::Search(search) => {
                self.search.handle(&mut ctx, search)?;
            }
            
            AppAction::Conflict(conflict) => {
                self.conflict.handle(&mut ctx, conflict)?;
            }
            
            AppAction::Edit(edit) => {
                self.edit.handle(&mut ctx, edit)?;
            }
            
            AppAction::SwitchView(mode) => {
                self.state.view_mode = mode;
            }
            
            AppAction::ToggleHelp => {
                if self.state.view_mode == crate::state::ViewMode::Help {
                    self.state.view_mode = crate::state::ViewMode::Graph;
                } else {
                    self.state.view_mode = crate::state::ViewMode::Help;
                }
            }
            
            AppAction::CopyToClipboard => {
                self.clipboard.handle(&mut ctx, ())?;
            }
            
            AppAction::None => {}
        }

        Ok(())
    }
}
```

---

## 8. Handler de conflits (sous-module)

### Fichier: `src/handler/conflict/mod.rs`

```rust
//! Gestion de la résolution de conflits de merge.

mod navigation;
mod resolution;
mod editing;
mod finalize;

use crate::error::Result;
use crate::state::AppState;
use crate::state::action::ConflictAction;
use super::traits::{ActionHandler, HandlerContext};

/// Handler pour la résolution de conflits.
pub struct ConflictHandler {
    navigation: navigation::ConflictNavigationHandler,
    resolution: resolution::ConflictResolutionHandler,
    editing: editing::ConflictEditingHandler,
    finalize: finalize::ConflictFinalizeHandler,
}

impl ConflictHandler {
    pub fn new() -> Self {
        Self {
            navigation: navigation::ConflictNavigationHandler,
            resolution: resolution::ConflictResolutionHandler,
            editing: editing::ConflictEditingHandler,
            finalize: finalize::ConflictFinalizeHandler,
        }
    }
}

impl ActionHandler for ConflictHandler {
    type Action = ConflictAction;

    fn can_handle(&self, state: &AppState, _action: &Self::Action) -> bool {
        state.conflicts_state.is_some()
    }

    fn handle(&mut self, ctx: &mut HandlerContext, action: ConflictAction) -> Result<()> {
        match action {
            // Navigation
            ConflictAction::PreviousFile |
            ConflictAction::NextFile |
            ConflictAction::PreviousSection |
            ConflictAction::NextSection |
            ConflictAction::SwitchPanel => {
                self.navigation.handle(ctx, action)
            }
            
            // Résolution
            ConflictAction::AcceptOursFile |
            ConflictAction::AcceptTheirsFile |
            ConflictAction::AcceptOursBlock |
            ConflictAction::AcceptTheirsBlock |
            ConflictAction::AcceptBoth |
            ConflictAction::MarkResolved => {
                self.resolution.handle(ctx, action)
            }
            
            // Édition
            ConflictAction::StartEdit |
            ConflictAction::ConfirmEdit |
            ConflictAction::CancelEdit => {
                self.editing.handle(ctx, action)
            }
            
            // Finalisation
            ConflictAction::FinalizeMerge |
            ConflictAction::AbortMerge => {
                self.finalize.handle(ctx, action)
            }
        }
    }
}
```

---

## 9. Plan de migration

### Phase 1: Préparation
1. Créer la structure de dossiers `src/handler/`
2. Créer `traits.rs` et `helpers.rs`
3. Compiler pour vérifier

### Phase 2: Extraire les handlers simples
1. `navigation.rs` - Le plus simple, peu de dépendances
2. `clipboard.rs` - Isolé
3. `search.rs` - Relativement isolé

### Phase 3: Extraire les handlers git
1. `staging.rs` 
2. `branch.rs`
3. `stash.rs`
4. `remote.rs`

### Phase 4: Extraire conflict handler
1. Créer le sous-module `conflict/`
2. Migrer les 935 lignes en 4 fichiers
3. Tester minutieusement

### Phase 5: Finaliser
1. Créer `mod.rs` avec le dispatcher
2. Supprimer l'ancien `event.rs`
3. Mettre à jour les imports dans `app.rs`

---

## 10. Checklist de validation

```bash
# 1. Créer la structure
tree src/handler/

# 2. Compilation incrémentale après chaque fichier
cargo check

# 3. Tests
cargo test

# 4. Vérifier qu'event.rs est supprimé
[ ! -f src/event.rs ] && echo "OK: event.rs supprimé"

# 5. Compter les lignes
find src/handler -name "*.rs" -exec wc -l {} + | tail -1
# Devrait être ~3400 lignes réparties en ~15 fichiers

# 6. Clippy
cargo clippy --all-features -- -D warnings

# 7. Test complet de l'application
cargo run
# Tester: navigation, staging, branches, conflicts, search, etc.
```

---

## Bénéfices attendus

| Métrique | Avant | Après |
|----------|-------|-------|
| Taille du plus gros fichier | 3400 lignes | ~400 lignes |
| Fichiers handler | 1 | 15 |
| Couplage | Élevé | Faible |
| Testabilité unitaire | Impossible | Possible |
| Temps pour trouver un handler | Long | Court |
| Possibilité d'ajouter un nouveau handler | Difficile | Facile |
