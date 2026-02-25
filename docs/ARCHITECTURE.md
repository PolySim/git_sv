# Architecture de git_sv

## Objectif

`git_sv` est un outil CLI interactif (TUI) en Rust permettant de visualiser le graphe git
d'un repository directement dans le terminal, et d'effectuer des opérations git courantes
(commit, merge, stash, branches, cherry-pick, blame, résolution de conflits, etc.).

---

## Stack technique

| Crate       | Version | Rôle                                          |
|-------------|---------|-----------------------------------------------|
| ratatui     | 0.29    | Framework TUI (rendu terminal)                |
| crossterm   | 0.28    | Backend terminal (compatible macOS/Linux/Win) |
| git2        | 0.19    | Bindings libgit2 (opérations git)             |
| clap        | 4       | Parsing des arguments CLI (derive)            |
| anyhow      | 1       | Gestion d'erreurs applicatives                |
| thiserror   | 2       | Erreurs typées custom                         |
| chrono      | 0.4     | Formatage des dates de commits                |
| arboard     | 3       | Accès au presse-papier système                |
| lru         | 0.12    | Cache LRU pour les diffs                      |
| notify      | 6       | Surveillance du système de fichiers           |
| tempfile    | 3       | Répertoires temporaires (tests)               |

---

## Structure du projet

```
git_sv/
├── Cargo.toml
├── AGENTS.md                  # Instructions pour les AI agents
├── docs/
│   ├── ARCHITECTURE.md        # Ce fichier
│   ├── CONTRIBUTING.md        # Guide pour les contributeurs
│   └── STEP-XX-*.md           # Feuille de route et plans d'amélioration
└── src/
    ├── main.rs                # Point d'entrée, parsing CLI (clap)
    ├── app.rs                 # Orchestration : initialisation et lancement
    ├── error.rs               # Types d'erreurs custom (thiserror)
    ├── error_display.rs       # Utilitaires d'affichage des erreurs
    ├── terminal.rs            # Setup/teardown du terminal crossterm
    ├── watcher.rs             # Surveillance des changements git (polling)
    ├── git/                   # Couche d'accès git (libgit2)
    ├── handler/               # Gestionnaires d'actions (EventLoop)
    ├── state/                 # État global de l'application
    ├── ui/                    # Rendu des vues (ratatui)
    ├── utils/                 # Utilitaires généraux
    └── test_utils/            # Helpers pour les tests
```

---

## Module `git/` — Couche d'accès git

```
src/git/
├── mod.rs         # Re-exports publics
├── repo.rs        # Wrapper GitRepo autour de git2::Repository
├── graph.rs       # Construction du graphe de commits (colonnes, connexions)
├── commit.rs      # Création et amendement de commits
├── branch.rs      # Opérations branches (list, create, checkout, delete, rename)
├── stash.rs       # Opérations stash (list, save, pop, drop)
├── merge.rs       # Merge avec détection de conflits
├── diff.rs        # Types et parsing des diffs (unifié, side-by-side)
├── blame.rs       # Blame par fichier et ligne
├── conflict.rs    # Résolution de conflits (bloc, ligne, fichier)
├── discard.rs     # Discard des modifications (fichier ou tout)
├── helpers.rs     # Fonctions utilitaires git
├── remote.rs      # Opérations remote (push, pull, fetch)
├── search.rs      # Recherche dans les commits
├── worktree.rs    # Gestion des worktrees
└── tests/
    └── test_utils.rs  # Helpers pour créer des repos de test
```

### Types clés

| Type          | Fichier       | Description                                          |
|---------------|---------------|------------------------------------------------------|
| `GitRepo`     | `repo.rs`     | Wrapper autour de `git2::Repository`                 |
| `CommitNode`  | `graph.rs`    | Nœud du graphe (oid, message, auteur, colonne, etc.) |
| `GraphRow`    | `graph.rs`    | Ligne du graphe (CommitNode + cellules visuelles)    |
| `ConnectionRow` | `graph.rs`  | Ligne de connexion entre deux commits                |
| `DiffFile`    | `diff.rs`     | Fichier modifié dans un commit                       |
| `FileDiff`    | `diff.rs`     | Contenu du diff d'un fichier                         |
| `DiffLine`    | `diff.rs`     | Ligne de diff (ajout, suppression, contexte)         |
| `StatusEntry` | `repo.rs`     | Entrée de statut (staged, unstaged, untracked)       |
| `BranchInfo`  | `branch.rs`   | Informations sur une branche (nom, is_head, remote)  |
| `StashEntry`  | `stash.rs`    | Entrée de stash                                      |
| `WorktreeInfo`| `worktree.rs` | Informations sur un worktree                         |

---

## Module `handler/` — Gestionnaires d'actions

```
src/handler/
├── mod.rs          # Re-exports + EventHandler (boucle événementielle)
├── dispatcher.rs   # ActionDispatcher : routing des AppAction vers handlers
├── traits.rs       # Traits ActionHandler et HandlerContext
├── navigation.rs   # Déplacements, sélection, scroll
├── git.rs          # Opérations git (commit, stash, merge, blame, push...)
├── staging.rs      # Staging/unstaging, commit, amend
├── branch.rs       # Gestion des branches et worktrees
├── conflict.rs     # Résolution de conflits
├── search.rs       # Recherche dans les commits
├── edit.rs         # Édition de texte (input fields)
└── filter.rs       # Filtrage du graphe
```

### Architecture du dispatcher

```
EventHandler (boucle)
  └─ handle_input_with_timeout()    ← crossterm events
       └─ map_key() / map_mouse()   ← src/ui/input.rs
            └─ AppAction
  └─ ActionDispatcher::dispatch()
       ├─ NavigationHandler
       ├─ GitHandler
       ├─ StagingHandler
       ├─ BranchHandler
       ├─ ConflictHandler
       ├─ SearchHandler
       ├─ EditHandler
       └─ FilterHandler
```

Chaque handler implémente le trait `ActionHandler<Action = XxxAction>` et reçoit
un `HandlerContext<'a>` donnant accès mutable à l'`AppState`.

---

## Module `state/` — État global

```
src/state/
├── mod.rs          # AppState (struct centrale), MAX_COMMITS, constantes
├── action/
│   ├── mod.rs      # AppAction (enum principale) + re-exports
│   ├── navigation.rs
│   ├── git.rs
│   ├── staging.rs
│   ├── branch.rs
│   ├── conflict.rs
│   ├── search.rs
│   ├── edit.rs
│   └── filter.rs
├── view/
│   ├── mod.rs      # ViewMode, BottomLeftMode, FocusPanel
│   ├── graph.rs    # GraphViewState
│   ├── staging.rs  # StagingState, StagingFocus
│   ├── branches.rs # BranchesViewState, BranchesSection, BranchesFocus
│   ├── conflicts.rs# ConflictsState, ConflictPanelFocus
│   ├── blame.rs    # BlameState
│   ├── merge_picker.rs # MergePickerState
│   └── search.rs   # SearchState
├── cache.rs        # DiffCache (LRU), LazyDiff, LazyBlame
├── filter.rs       # GraphFilter, FilterPopupState, FilterField
└── selection.rs    # ListSelection<T> (sélection générique avec index)
```

### Modes de vue (`ViewMode`)

| Mode        | Description                                     |
|-------------|-------------------------------------------------|
| `Graph`     | Vue principale : graphe git + détails commit    |
| `Staging`   | Vue de staging/commit                           |
| `Branches`  | Vue branches / worktrees / stashes              |
| `Conflicts` | Vue de résolution de conflits                   |
| `Blame`     | Vue blame d'un fichier                          |
| `Help`      | Overlay d'aide (s'affiche par-dessus la vue actuelle) |

### `AppState` — Champs principaux

| Champ                  | Type                | Description                                  |
|------------------------|---------------------|----------------------------------------------|
| `repo`                 | `GitRepo`           | Repository git                               |
| `view_mode`            | `ViewMode`          | Vue active                                   |
| `graph`                | `Vec<GraphRow>`     | Commits chargés                              |
| `graph_view`           | `GraphViewState`    | État de la vue graph (sélection, scroll)     |
| `selected_index`       | `usize`             | Index du commit sélectionné                  |
| `staging_state`        | `StagingState`      | État de la vue staging                       |
| `branches_view_state`  | `BranchesViewState` | État de la vue branches                      |
| `blame_state`          | `Option<BlameState>` | État du blame (si actif)                    |
| `conflicts_state`      | `Option<ConflictsState>` | État de résolution de conflits          |
| `search_state`         | `SearchState`       | État de la recherche                         |
| `merge_picker`         | `Option<MergePickerState>` | Picker de merge (si actif)            |
| `diff_cache`           | `DiffCache`         | Cache LRU des diffs (capacité : 50 entrées)  |
| `graph_filter`         | `GraphFilter`       | Filtres actifs sur le graphe                 |
| `flash_message`        | `Option<(String, Instant)>` | Message temporaire (3s)            |
| `pending_confirmation` | `Option<ConfirmAction>` | Dialogue de confirmation en attente      |
| `dirty`                | `bool`              | Flag indiquant qu'un refresh est nécessaire  |

---

## Module `ui/` — Rendu des vues

```
src/ui/
├── mod.rs              # Point d'entrée render(), dispatch par ViewMode
├── input.rs            # Mapping clavier → AppAction (par mode de vue)
├── layout.rs           # Disposition des panneaux (ratatui Layout)
├── theme.rs            # Définition des couleurs et styles
├── graph_view.rs       # Rendu du graphe git
├── detail_view.rs      # Panneau détail commit
├── diff_view.rs        # Rendu du diff (unifié et side-by-side)
├── files_view.rs       # Liste des fichiers d'un commit
├── staging_layout.rs   # Disposition de la vue staging
├── staging_view.rs     # Rendu de la vue staging
├── branches_layout.rs  # Disposition de la vue branches
├── branches_view.rs    # Rendu de la vue branches
├── branch_panel.rs     # Panneau branches (legacy overlay)
├── conflicts_view.rs   # Rendu de la vue conflits
├── blame_view.rs       # Rendu de la vue blame
├── nav_bar.rs          # Barre de navigation (tabs de vue)
├── status_bar.rs       # Barre de statut (branche, repo, flash)
├── help_bar.rs         # Barre d'aide contextuelle en bas
├── help_overlay.rs     # Overlay d'aide complète (?)
├── search_bar.rs       # Barre de recherche
├── graph_legend.rs     # Légende du graphe
├── filter_popup.rs     # Popup de filtrage du graphe
├── merge_picker.rs     # Picker de branche pour merge
├── confirm_dialog.rs   # Dialogue de confirmation (actions destructives)
├── loading.rs          # Spinner de chargement
└── common/             # Composants UI réutilisables
    ├── mod.rs          # Re-exports + StatusBarConfig + render_status_bar()
    ├── block.rs        # StyledBlock (bloc avec titre et bordures)
    ├── help_bar.rs     # HelpBar, KeyBinding
    ├── list.rs         # StyledList, list_item, list_item_styled
    ├── popup.rs        # Popup (centrage et rendu d'overlay)
    ├── rect.rs         # centered_rect, is_terminal_size_adequate
    ├── style.rs        # Styles et constantes de couleur
    └── text.rs         # truncate, pad_left, pad_right, etc.
```

---

## Module `utils/` — Utilitaires

```
src/utils/
├── mod.rs       # Re-exports
└── time.rs      # Formatage des timestamps git
```

---

## Module `test_utils/` — Helpers de test

```
src/test_utils/
├── mod.rs            # Re-exports
├── mock_repo.rs      # Création de repos git temporaires pour les tests
├── test_state.rs     # Création d'AppState de test
└── assertions.rs     # Assertions spécialisées pour les tests
```

---

## Flux de données

```
┌──────────────────────────────────────────────────────────────────┐
│                         main.rs (CLI/clap)                        │
└───────────────────────────────┬──────────────────────────────────┘
                                │
                                v
┌──────────────────────────────────────────────────────────────────┐
│                          app.rs (App)                             │
│   Initialisation : git data, staging, graph, worktrees            │
└───────────────────────────────┬──────────────────────────────────┘
                                │
                                v
┌──────────────────────────────────────────────────────────────────┐
│                 handler/mod.rs (EventHandler)                     │
│                                                                   │
│  loop {                                                           │
│    terminal.draw(|f| ui::render(f, &state))                      │
│    watcher.check_changed() → state.dirty                         │
│    handle_input_with_timeout() → Option<AppAction>               │
│    dispatcher.dispatch(&mut state, action)                        │
│    state.check_flash_expired()                                    │
│    if state.dirty { self.refresh() }                              │
│  }                                                                │
└────────┬──────────────────┬─────────────────┬────────────────────┘
         │                  │                  │
         v                  v                  v
┌──────────────┐  ┌──────────────────┐  ┌────────────┐
│  ui/input.rs │  │handler/dispatcher│  │  git/      │
│  map_key()   │  │ActionDispatcher  │  │  (GitRepo) │
│  map_mouse() │  │  → Handlers      │  └────────────┘
└──────────────┘  └──────────────────┘
```

### Cycle de rendu

1. `ui::render(frame, &mut state)` dispatche vers la vue active (`ViewMode`)
2. Chaque vue appelle `layout::build_layout()` pour diviser l'espace
3. Les composants reçoivent des `Rect` et `&state` en lecture seule
4. Le `theme.rs` fournit les couleurs et styles centralisés
5. `common/` fournit les widgets réutilisables (Popup, StyledList, etc.)

---

## Système de cache (LRU Diff Cache)

`DiffCache` est un cache LRU (Least Recently Used) de capacité 50 entrées.  
La clé est `DiffCacheKey { oid: Oid, path: String, is_working_dir: bool }`.

- Les diffs de commits sont mis en cache indéfiniment (tant qu'ils ne sont pas évictés).
- Les diffs du working directory sont invalidés lors d'un `mark_dirty()`.
- `LazyDiff` et `LazyBlame` permettent un chargement paresseux.

---

## File Watcher

`GitWatcher` (dans `watcher.rs`) surveille les changements git par polling des timestamps :

- Fichiers surveillés : `.git/HEAD`, `.git/index`, `.git/refs/heads/`
- Intervalle de vérification : 2 secondes
- Debounce : 500ms après la dernière modification détectée
- Quand un changement est détecté : `state.dirty = true` → `EventHandler::refresh()`

---

## Système de thème

`src/ui/theme.rs` centralise les couleurs de l'application.  
`src/ui/common/style.rs` exporte les styles ratatui réutilisables :

| Constante/Fonction    | Description                         |
|-----------------------|-------------------------------------|
| `FOCUS_COLOR`         | Couleur de bordure quand focalisé   |
| `INACTIVE_COLOR`      | Couleur de bordure quand inactif    |
| `highlight_style()`   | Style de sélection (fond coloré)    |
| `diff_add_style()`    | Style ajout (vert)                  |
| `diff_remove_style()` | Style suppression (rouge)           |
| `error_style()`       | Style erreur (rouge)                |
| `success_style()`     | Style succès (vert)                 |
| `dim_style()`         | Style grisé (inactive text)         |

---

## Keybindings par mode de vue

### Vue Graph (mode par défaut)

| Touche         | Action                                   |
|----------------|------------------------------------------|
| `q`            | Quitter                                  |
| `j` / `↓`     | Commit suivant                           |
| `k` / `↑`     | Commit précédent                         |
| `g` / `Home`   | Premier commit                           |
| `G` / `End`    | Dernier commit                           |
| `Ctrl+d`       | Page suivante                            |
| `Ctrl+u`       | Page précédente                          |
| `Enter`        | Focus sur la liste de fichiers           |
| `Tab`          | Basculer le focus entre les panneaux     |
| `c`            | Nouveau commit (ouvre la vue Staging)    |
| `s`            | Stash                                    |
| `m`            | Merge (ouvre le picker de branche)       |
| `b`            | Panneau branches (legacy)                |
| `P`            | Push                                     |
| `p`            | Pull                                     |
| `f`            | Fetch                                    |
| `x`            | Cherry-pick                              |
| `B`            | Blame du fichier sélectionné             |
| `/`            | Ouvrir la recherche                      |
| `n` / `N`      | Résultat suivant / précédent             |
| `F`            | Ouvrir le popup de filtre                |
| `Ctrl+r`       | Effacer les filtres actifs               |
| `r`            | Rafraîchir                               |
| `y`            | Copier dans le presse-papier             |
| `v`            | Basculer mode diff (unifié/side-by-side) |
| `M`            | Basculer mode panneau bas-gauche         |
| `?`            | Aide                                     |
| `1/2/3/4`      | Changer de vue (Graph/Staging/Branches/Conflicts) |

### Vue Staging

| Touche         | Action                                   |
|----------------|------------------------------------------|
| `j` / `↓`     | Fichier suivant                          |
| `k` / `↑`     | Fichier précédent                        |
| `s` / `Enter`  | Stager le fichier (panneau Unstaged)     |
| `u` / `Enter`  | Unstager le fichier (panneau Staged)     |
| `a`            | Stager tous les fichiers                 |
| `U`            | Unstager tous les fichiers               |
| `d`            | Discard le fichier sélectionné           |
| `D`            | Discard toutes les modifications         |
| `c`            | Saisir le message de commit              |
| `A`            | Amend le dernier commit                  |
| `Tab`          | Basculer le focus (Unstaged/Staged/Diff) |
| `Enter` (commit)| Confirmer le commit                    |
| `Esc`          | Annuler                                  |

### Vue Branches

| Touche         | Action                                   |
|----------------|------------------------------------------|
| `j` / `↓`     | Branche / worktree / stash suivant       |
| `k` / `↑`     | Précédent                                |
| `Tab`          | Section suivante (Branches/Worktrees/Stashes) |
| `Enter`        | Checkout la branche sélectionnée         |
| `n`            | Créer une nouvelle branche / worktree    |
| `d`            | Supprimer                                |
| `r`            | Renommer la branche                      |
| `R`            | Afficher/masquer les branches remote     |
| `m`            | Merger la branche sélectionnée           |
| `a`            | Appliquer le stash                       |
| `p`            | Pop le stash                             |

### Vue Conflicts

| Touche         | Action                                   |
|----------------|------------------------------------------|
| `Tab`          | Basculer le panneau actif                |
| `j` / `k`      | Navigation (fichier, section, ou ligne)  |
| `o` / `←`      | Accepter "ours" (dans FileList)          |
| `t` / `→`      | Accepter "theirs" (dans FileList)        |
| `b`            | Accepter les deux (mode Bloc)            |
| `Space`        | Toggle sélection / entrée résolution     |
| `Enter`        | Valider la résolution                    |
| `r`            | Marquer comme résolu                     |
| `i` / `e`      | Mode édition (panneau résultat)          |
| `F/B/L`        | Changer le mode de résolution            |
| `V`            | Finaliser le merge                       |
| `A`            | Annuler le merge                         |
| `q` / `Esc`    | Quitter la vue conflits                  |

### Vue Blame

| Touche         | Action                                   |
|----------------|------------------------------------------|
| `j` / `↓`     | Ligne suivante                           |
| `k` / `↑`     | Ligne précédente                         |
| `g` / `Home`   | Première ligne                           |
| `G` / `End`    | Dernière ligne                           |
| `Enter`        | Sauter au commit blame                   |
| `y`            | Copier dans le presse-papier             |
| `q` / `Esc`    | Fermer le blame                          |

---

## Patterns et conventions

### Gestion d'erreurs

```rust
// Types d'erreurs dans src/error.rs
pub enum GitSvError {
    Git(git2::Error),
    Io(std::io::Error),
    Clipboard(String),
    // ...
}

// Alias dans src/error.rs
pub type Result<T> = std::result::Result<T, GitSvError>;

// Usage avec contexte (src/error_display.rs)
file.open(path).with_context(|| format!("Ouverture de {}", path))?
```

### Pattern ListSelection

`ListSelection<T>` (dans `state/selection.rs`) est un wrapper générique
autour de `Vec<T>` avec gestion de l'index sélectionné. Il remplace
l'usage direct de `ratatui::widgets::ListState` dans l'état métier.

```rust
let mut list = ListSelection::with_items(items);
list.select(0);
let item = list.selected_item(); // Option<&T>
list.move_down();
list.move_up();
```

### Messages flash

Les messages flash sont affichés dans la status bar pendant 3 secondes :
```rust
state.set_flash_message("Opération réussie ✓");
// Vérification automatique dans EventHandler::run()
state.check_flash_expired();
```

### Dialogue de confirmation

Pour les actions destructives, utiliser `ConfirmAction` :
```rust
state.pending_confirmation = Some(ConfirmAction::DiscardFile(path));
// L'utilisateur doit taper y/n
// Le dispatcher handle AppAction::ConfirmAction / AppAction::CancelAction
```
