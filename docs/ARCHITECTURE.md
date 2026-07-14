# Architecture de git_sv

## Objectif

`git_sv` est un outil Rust qui combine un mode CLI non interactif et une TUI pour explorer un repository git, afficher son historique sous forme de graphe et executer les operations courantes du quotidien.

---

## Stack technique

| Crate            | Version | Role |
|------------------|---------|------|
| ratatui          | 0.29    | Rendu TUI |
| crossterm        | 0.28    | Evenements clavier/souris et backend terminal |
| git2             | 0.19    | Acces aux operations git via libgit2 |
| clap             | 4       | Parsing CLI |
| anyhow           | 1       | Erreurs applicatives au point d'entree |
| thiserror        | 2       | Erreurs typees du projet |
| chrono           | 0.4     | Dates et formatage temporel |
| dirs             | 5       | Localisation du fichier de configuration utilisateur |
| arboard          | 3       | Presse-papier |
| lru              | 0.12    | Cache des diffs |
| serde / serde_json | 1.0   | Sorties JSON du CLI |
| tempfile         | 3       | Repositories temporaires pour les tests |
| insta            | 1.34    | Support de snapshots de tests |

Le watcher git n'utilise pas `notify` : il repose sur un polling leger des fichiers critiques du repertoire `.git` dans `src/watcher.rs`.

---

## Structure du projet

```text
git_sv/
├── Cargo.toml
├── README.md
├── docs/
│   ├── ARCHITECTURE.md
│   ├── PERFORMANCE.md
│   └── CONTRIBUTING.md
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── error.rs
│   ├── i18n.rs
│   ├── terminal.rs
│   ├── watcher.rs
│   ├── git/
│   │   ├── conflict/
│   │   ├── graph/
│   │   ├── remote/
│   │   └── tests/
│   ├── handler/
│   │   ├── branch/
│   │   ├── conflict/
│   │   ├── dispatcher/
│   │   ├── navigation/
│   │   └── staging/
│   ├── state/
│   │   ├── action/
│   │   └── view/
│   ├── test_utils/
│   ├── ui/
│   │   ├── common/
│   │   ├── conflicts_view/
│   │   ├── graph_view/
│   │   └── input/
│   └── utils/
├── tests/
│   ├── common/
│   ├── integration/
│   └── integration_test.rs
└── homebrew/
    └── git_sv.rb
```

---

## Point d'entree

- `src/main.rs` parse les arguments, ouvre le repository et choisit entre mode CLI et TUI.
- `src/cli.rs` implemente les commandes non interactives : `log`, `branches`, `status`, `inspect`, `search`, `graph`.
- `src/app.rs` initialise l'etat TUI, charge les donnees initiales et lance la boucle evenementielle.

### Flux global

```text
main.rs
  -> GitRepo::open()
  -> commande CLI directe via cli.rs
     ou
  -> App::new()
  -> EventHandler::run()
  -> ui::render()
```

---

## Couche git

Le dossier `src/git/` encapsule les operations metier sur git :

```text
src/git/
├── mod.rs
├── bisect.rs
├── repo.rs
├── graph.rs
├── graph/
├── commit.rs
├── branch.rs
├── stash.rs
├── merge.rs
├── diff.rs
├── external_diff.rs
├── custom_command.rs
├── github.rs
├── insights.rs
├── reflog.rs
├── tag.rs
├── blame.rs
├── discard.rs
├── helpers.rs
├── remote.rs
├── remote/
├── search.rs
├── worktree.rs
└── conflict/
    ├── mod.rs
    ├── content.rs
    ├── merge_files.rs
    ├── parse.rs
    ├── repo_state.rs
    ├── resolve.rs
    └── types.rs
```

### Types importants

| Type | Fichier | Description |
|------|---------|-------------|
| `GitRepo` | `src/git/repo.rs` | Facade principale autour de `git2::Repository` |
| `CommitInfo` | `src/git/commit.rs` | Representation CLI d'un commit |
| `CommitNode` | `src/git/graph.rs` | Noeud du graphe pour la TUI |
| `GraphRow` | `src/git/graph.rs` | Ligne de rendu du graphe |
| `BranchInfo` | `src/git/branch.rs` | Metadonnees de branche locale ou distante |
| `StatusEntry` | `src/git/repo.rs` | Entree de statut pour staging et CLI |
| `StashEntry` | `src/git/stash.rs` | Entree de stash |
| `WorktreeInfo` | `src/git/worktree.rs` | Entree de worktree |
| `ProjectTreeEntry` | `src/state/view/project_tree.rs` | Fichier ou dossier visible dans l'arborescence |

---

## Etat applicatif

`src/state/` centralise l'etat metier de la TUI.

```text
src/state/
├── mod.rs
├── action/
│   ├── mod.rs
│   ├── branch.rs
│   ├── conflict.rs
│   ├── edit.rs
│   ├── filter.rs
│   ├── git.rs
│   ├── navigation.rs
│   ├── project_tree.rs
│   ├── search.rs
│   └── staging.rs
├── cache.rs
├── filter.rs
├── selection.rs
└── view/
    ├── mod.rs
    ├── blame.rs
    ├── branches.rs
    ├── conflicts.rs
    ├── graph.rs
    ├── merge_picker.rs
    ├── project_tree.rs
    ├── reset_picker.rs
    ├── search.rs
    └── staging.rs
```

### `AppState`

`AppState` vit dans `src/state/mod.rs` et regroupe notamment :

- le repository ouvert et son chemin ;
- le mode de vue courant ;
- `graph_view`, qui contient le graphe, les selections et l'etat du diff ;
- les etats de vues secondaires : staging, branches, conflits, blame, recherche ;
- `project_tree_state`, qui contient l'arborescence repliable, la recherche floue locale,
  l'historique du chemin selectionne, les fichiers du commit et leur diff ;
- les messages flash, boites de confirmation et pickers temporaires ;
- le cache des diffs et les filtres appliques ;
- la vue branches repose sur `BranchesViewState`, sans overlay dedie en vue graph.

### Modes principaux

`ViewMode` dans `src/state/view/mod.rs` expose :

- `Graph`
- `Staging`
- `Branches`
- `ProjectTree`
- `Conflicts`
- `Blame`
- `Help`

---

## Boucle evenementielle et handlers

Le module `src/handler/` porte la boucle principale et le dispatch des actions.

```text
src/handler/
├── mod.rs
├── background.rs
├── branch.rs
├── branch/
├── edit.rs
├── filter.rs
├── git.rs
├── navigation.rs
├── navigation/
├── search.rs
├── staging.rs
├── staging/
├── traits.rs
├── conflict/
│   ├── mod.rs
│   ├── edit.rs
│   ├── modes.rs
│   ├── navigation.rs
│   └── shared.rs
└── dispatcher/
    ├── mod.rs
    ├── clipboard.rs
    ├── confirmations.rs
    ├── pickers.rs
    └── tests.rs
```

### Pipeline d'execution

```text
EventHandler::run()
  -> ui::render(frame, &mut state)
  -> GitWatcher::check_changed()
  -> ui::input::handle_input_with_timeout()
  -> AppAction
  -> ActionDispatcher::dispatch()
  -> mutation de AppState
  -> refresh conditionnel
```

### Background jobs

Les operations reseau potentiellement longues (`push`, `pull`, `fetch` et lecture de PR GitHub) sont lancees via `src/handler/background.rs` avec spinner de chargement et restitution du resultat dans la boucle principale. Le rebase interactif, le difftool et les commandes utilisateur suspendent explicitement le terminal TUI avant de céder la main au processus externe.

---

## UI

Le rendu TUI reside dans `src/ui/`.

```text
src/ui/
├── mod.rs
├── blame_view.rs
├── branches_layout.rs
├── branches_view.rs
├── confirm_dialog.rs
├── conflicts_view.rs
├── conflicts_view/
├── detail_view.rs
├── diff_view.rs
├── files_view.rs
├── filter_popup.rs
├── graph_view/
├── help_bar.rs
├── help_overlay.rs
├── hit_test.rs
├── input/
│   ├── mod.rs
│   ├── keyboard.rs
│   ├── mouse.rs
│   └── tests.rs
├── keybindings.rs
├── layout.rs
├── loading.rs
├── merge_picker.rs
├── nav_bar.rs
├── reset_picker.rs
├── search_bar.rs
├── staging_layout.rs
├── staging_view.rs
├── status_bar.rs
├── theme.rs
└── common/
    ├── mod.rs
    ├── block.rs
    ├── help_bar.rs
    ├── list.rs
    ├── popup.rs
    ├── rect.rs
    ├── style.rs
    └── text.rs
```

### Organisation

- `ui/mod.rs` orchestre le rendu selon `ViewMode`.
- `ui/input/keyboard.rs` et `ui/input/mouse.rs` traduisent les interactions en `AppAction`.
- `layout.rs`, `branches_layout.rs` et `staging_layout.rs` decoupent l'ecran.
- `common/` contient des composants et helpers reutilisables.
- `branches_view.rs` porte toute l'interface de navigation branches/worktrees/stashes.

---

## CLI non interactif

`src/cli.rs` propose trois formats de sortie :

- `human`
- `plain`
- `json`

`--format` est une option globale de `git_sv`, pas une option propre a chaque sous-commande.

La sortie JSON existe pour `log`, `branches`, `status` et `graph`.

La sous-commande `graph` borne actuellement la sortie a 50 commits pour eviter les sorties trop lourdes.

Le format `graph` JSON expose une vue simplifiee de chaque commit : hash, message, auteur, parents, colonne, couleur et references attachees.

---

## Watcher git

Le watcher dans `src/watcher.rs` surveille par polling :

- `.git/HEAD`
- `.git/index`
- `.git/refs/heads/`
- `.git/packed-refs`
- un snapshot des fichiers modifies du working tree

Regles actuelles :

- métadonnées Git : 2 secondes ;
- working tree : 5 secondes ;
- debounce : 500 ms ;
- si un changement est confirme, `state.dirty` est releve puis la boucle recharge les donnees.

Le watcher gere aussi les worktrees de maniere best-effort via `git rev-parse --git-dir` si `.git/` n'est pas un repertoire direct.

---

## Cache des diffs

`DiffCache` dans `src/state/cache.rs` stocke les diffs les plus recents via un LRU.

- capacite actuelle : 50 entrees ;
- budget mémoire estimé : 64 MiB ;
- valeurs partagées avec `Arc<FileDiff>` pour éviter les copies ;
- les diffs du working directory sont invalides lors d'un `mark_dirty()` ;
- les diffs de commits restent en cache jusqu'a eviction.

---

## Tests

Le projet combine :

- des tests unitaires co-localises dans les modules ;
- des helpers de test dans `src/test_utils/` et `src/git/tests/test_utils.rs` ;
- des tests d'integration dans `tests/`.

Structure actuelle :

```text
src/test_utils/
├── assertions.rs
├── mod.rs
└── ui_driver.rs

tests/
├── common/
├── integration/
└── integration_test.rs
```

---

## Points a surveiller

- Quelques `allow(dead_code)` restent presents dans les zones de compatibilite et dans certains utilitaires UI encore partiellement integres.
- La release publie Homebrew et Scoop via des depots externes ; `homebrew/git_sv.rb` dans ce repo sert de gabarit local desactive.
