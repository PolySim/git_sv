# Phase 1 — Refactorisation structurelle

## 1.1 Découper `app.rs` (God Object — 1114 lignes) ✅

**Problème** : `app.rs` contient l'état complet de l'application, toute la logique métier, la boucle événementielle et la gestion du terminal. La méthode `apply_action()` fait ~500 lignes à elle seule avec un `match` géant.

**Étapes** :

- [x] **Extraire `AppState` dans `state.rs`** : Séparer l'état pur (données) de la logique.
  - Déplacé dans `src/state.rs` : `StagingState`, `BranchesViewState`, `BranchesSection`, `BranchesFocus`, `InputAction`, `StagingFocus`, `FocusPanel`, `BottomLeftMode`, `ViewMode`, `AppAction`.
  - L'App ne garde que l'`AppState` et coordonne les composants.

- [x] **Découper `apply_action()` en sous-méthodes thématiques** :
  - Toutes les actions sont maintenant dans `src/event.rs` avec `EventHandler` :
    - `handle_move_up/down()`, `handle_page_up/down()`, `handle_go_top/bottom()` - Navigation
    - `handle_stage_file()`, `handle_unstage_file()`, `handle_stage_all()`, `handle_unstage_all()` - Staging
    - `handle_branch_checkout()`, `handle_branch_rename()` - Branches
    - `handle_stash_apply()`, `handle_stash_pop()`, `handle_stash_drop()` - Stash
    - `handle_insert_char()`, `handle_delete_char()`, `handle_move_cursor_*()` - Input
    - `handle_switch_to_*()` - View switching

- [x] **Extraire la gestion du terminal** : Déplacé dans `src/terminal.rs` avec `setup_terminal()` et `restore_terminal()`.

- [x] **Extraire la boucle événementielle** : Créé `src/event.rs` avec `EventHandler` qui gère le rendu, le polling et les actions.

## 1.2 Réduire la signature de `ui::render()` (19 paramètres) ✅

**Problème** : La fonction `render()` dans `ui/mod.rs` prend 19 paramètres. C'est un code smell majeur.

**Étapes** :

- [x] **Passer une référence `&AppState` directement** : La fonction `render()` a maintenant la signature : `pub fn render(frame: &mut Frame, state: &AppState)`.
- [x] **Adapter toutes les sous-fonctions de rendu** : `render_graph_view()` reçoit `&AppState` et accède aux champs nécessaires.
- [x] **Supprimer `#[allow(clippy::too_many_arguments)]`** : Supprimé de `ui::render()`.

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
