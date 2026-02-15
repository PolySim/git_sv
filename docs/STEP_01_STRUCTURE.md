# Phase 1 — Refactorisation structurelle

## 1.1 Découper `app.rs` (God Object — 1114 lignes)

**Problème** : `app.rs` contient l'état complet de l'application, toute la logique métier, la boucle événementielle et la gestion du terminal. La méthode `apply_action()` fait ~500 lignes à elle seule avec un `match` géant.

**Étapes** :

- [ ] **Extraire `AppState` dans `state.rs`** : Séparer l'état pur (données) de la logique.
  - Déplacer `StagingState`, `BranchesViewState`, `BranchesSection`, `BranchesFocus`, `InputAction`, `StagingFocus`, `FocusPanel`, `BottomLeftMode`, `ViewMode` dans un module `src/state/` ou `src/state.rs`.
  - L'App ne devrait garder que les champs nécessaires à l'orchestration.

- [ ] **Découper `apply_action()` en sous-méthodes thématiques** :
  - `handle_navigation_action(&mut self, action) -> Result<()>` (MoveUp, MoveDown, PageUp, PageDown, GoTop, GoBottom)
  - `handle_staging_action(&mut self, action) -> Result<()>` (StageFile, UnstageFile, StageAll, etc.)
  - `handle_branch_action(&mut self, action) -> Result<()>` (BranchCheckout, BranchCreate, etc.)
  - `handle_stash_action(&mut self, action) -> Result<()>` (StashApply, StashPop, etc.)
  - `handle_input_action(&mut self, action) -> Result<()>` (InsertChar, DeleteChar, curseur, etc.)
  - `handle_view_action(&mut self, action) -> Result<()>` (SwitchToGraph, SwitchToStaging, etc.)

- [ ] **Extraire la gestion du terminal** : Déplacer `setup_terminal()` et `restore_terminal()` dans un module `src/terminal.rs`.

- [ ] **Extraire la boucle événementielle** : Créer un module `src/event.rs` avec un `EventHandler` qui gère le polling, la conversion d'événements et le tick rate.

## 1.2 Réduire la signature de `ui::render()` (19 paramètres)

**Problème** : La fonction `render()` dans `ui/mod.rs` prend 19 paramètres. C'est un code smell majeur.

**Étapes** :

- [ ] **Passer une référence `&App` directement** au lieu de destructurer tous les champs. La fonction `render()` devrait avoir la signature : `pub fn render(frame: &mut Frame, app: &App)`.
- [ ] **Adapter toutes les sous-fonctions de rendu** pour recevoir `&App` ou les sous-états pertinents (`&StagingState`, `&BranchesViewState`).
- [ ] **Supprimer `#[allow(clippy::too_many_arguments)]`** une fois la refactorisation terminée.

## 1.3 Restructurer les modules UI

**Problème** : Certains modules UI sont très spécialisés mais pas bien organisés.

**Étapes** :

- [ ] Regrouper les vues en sous-dossiers :
  ```
  ui/
  ├── mod.rs
  ├── common/          # Widgets réutilisables
  │   ├── mod.rs
  │   ├── centered_rect.rs
  │   ├── status_bar.rs
  │   └── help_bar.rs
  ├── graph/           # Vue Graph
  │   ├── mod.rs
  │   ├── graph_view.rs
  │   ├── detail_view.rs
  │   ├── diff_view.rs
  │   ├── files_view.rs
  │   └── layout.rs
  ├── staging/         # Vue Staging
  │   ├── mod.rs
  │   ├── staging_view.rs
  │   └── layout.rs
  ├── branches/        # Vue Branches
  │   ├── mod.rs
  │   ├── branches_view.rs
  │   └── layout.rs
  └── input.rs
  ```
