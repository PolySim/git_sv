# git_sv — Interface Git en Terminal

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Un client git interactif en terminal (TUI) avec graphe de commits style GitKraken, staging interactif, gestion des branches/worktrees/stashes, résolution de conflits, et bien plus.

![Rust](https://img.shields.io/badge/Rust-2021-orange) ![ratatui](https://img.shields.io/badge/TUI-ratatui-green)

## Fonctionnalités

- **Graphe de commits** — Visualisation style GitKraken avec lignes continues, couleurs stables par branche, et nœuds différenciés (commit, merge, sélection)
- **Staging interactif** — Stage/unstage fichier par fichier, visualisation des diffs, écriture du message de commit
- **Gestion des branches** — Créer, checkout, supprimer, renommer des branches locales et distantes
- **Résolution de conflits** — Vue 3 panneaux (ours/theirs/résultat) avec résolution bloc par bloc ou ligne par ligne
- **Git blame** — Annotation ligne par ligne d'un fichier
- **Worktrees** — Créer, lister, supprimer des worktrees
- **Stashes** — Sauvegarder, appliquer, pop, supprimer des stashes
- **Opérations distantes** — Push, pull, fetch avec support SSH
- **Recherche** — Rechercher des commits par message, auteur ou hash
- **Filtres** — Filtrer le graphe par auteur, date, chemin de fichier, message
- **Thème adaptatif** — Détection automatique du thème clair/sombre du terminal
- **File watcher** — Rafraîchissement automatique quand le repository change

---

## Installation

### Via Homebrew (macOS)

```bash
brew tap PolySim/tap
brew install git_sv
```

Mise à jour :

```bash
brew upgrade git_sv
```

### Via Scoop (Windows)

```bash
scoop bucket add git_sv https://github.com/PolySim/scoop-git_sv
scoop install git_sv
```

### Via cargo

```bash
cargo install --git https://github.com/PolySim/git_sv.git
```

### Depuis les sources

```bash
git clone https://github.com/PolySim/git_sv.git
cd git_sv
cargo build --release
./target/release/git_sv
```

> **Note** : Sur certains systèmes Linux, si la compilation échoue à cause d'OpenSSL, utilisez :
>
> ```bash
> cargo build --release --features vendored-ssl
> ```

---

## Utilisation

### Mode interactif (TUI)

```bash
# Dans un repository git
git_sv

# Spécifier un chemin
git_sv --path /chemin/vers/repo
git_sv -p /chemin/vers/repo
```

### Mode non-interactif (log)

```bash
# Afficher les 20 derniers commits
git_sv log

# Afficher les N derniers commits
git_sv log -n 50
git_sv log --max-count 50
```

---

## Raccourcis clavier

### Navigation entre les vues

| Touche   | Action                                      |
| -------- | ------------------------------------------- |
| `1`      | Vue Graph (historique des commits)          |
| `2`      | Vue Staging (staging et commits)            |
| `3`      | Vue Branches (branches, worktrees, stashes) |
| `?`      | Afficher/masquer l'aide complète            |
| `q`      | Quitter                                     |
| `Ctrl+c` | Quitter (force)                             |

### Vue Graph

**Navigation :**

| Touche       | Action           |
| ------------ | ---------------- |
| `j` / `↓`    | Commit suivant   |
| `k` / `↑`    | Commit précédent |
| `g` / `Home` | Premier commit   |
| `G` / `End`  | Dernier commit   |
| `Ctrl+d`     | Page suivante    |
| `Ctrl+u`     | Page précédente  |

**Panneaux :**

| Touche  | Action                                          |
| ------- | ----------------------------------------------- |
| `Tab`   | Cycle de focus : Graph → Fichiers → Détail/Diff |
| `Enter` | Sélectionner / entrer dans un panneau           |
| `Esc`   | Retour au panneau précédent                     |

**Actions :**

| Touche | Action                                  |
| ------ | --------------------------------------- |
| `b`    | Overlay liste des branches              |
| `r`    | Rafraîchir                              |
| `p`    | Push                                    |
| `P`    | Pull                                    |
| `f`    | Fetch                                   |
| `y`    | Copier le hash du commit                |
| `/`    | Ouvrir la recherche                     |
| `F`    | Ouvrir les filtres                      |
| `v`    | Toggle mode diff (unified/side-by-side) |

### Vue Staging

```
┌──────────────┬──────────────┐
│  Unstaged    │              │
├──────────────┤     Diff     │
│   Staged     │              │
├──────────────┴──────────────┤
│    Message de commit        │
└─────────────────────────────┘
```

| Contexte | Touche        | Action                         |
| -------- | ------------- | ------------------------------ |
| Unstaged | `s` / `Enter` | Stage le fichier sélectionné   |
| Unstaged | `a`           | Stage tous les fichiers        |
| Unstaged | `x`           | Discard le fichier             |
| Staged   | `u` / `Enter` | Unstage le fichier sélectionné |
| Staged   | `U`           | Unstage tout                   |
| Global   | `c`           | Activer le champ de message    |
| Message  | `Enter`       | Valider le commit              |
| Message  | `Esc`         | Annuler la saisie              |

### Vue Branches

3 onglets : **Branches** | **Worktrees** | **Stashes**

| Contexte  | Touche      | Action                      |
| --------- | ----------- | --------------------------- |
| Global    | `Tab`       | Section suivante            |
| Global    | `Shift+Tab` | Section précédente          |
| Branches  | `Enter`     | Checkout la branche         |
| Branches  | `n`         | Créer une branche           |
| Branches  | `d`         | Supprimer la branche        |
| Branches  | `r`         | Renommer la branche         |
| Branches  | `R`         | Toggle branches remote      |
| Branches  | `m`         | Merge dans la branche       |
| Worktrees | `n`         | Créer un worktree           |
| Worktrees | `d`         | Supprimer le worktree       |
| Stashes   | `s`         | Sauvegarder un stash        |
| Stashes   | `a`         | Appliquer (sans supprimer)  |
| Stashes   | `p`         | Pop (appliquer + supprimer) |
| Stashes   | `d`         | Supprimer le stash          |

---

## Caractéristiques du graphe

- **Lignes continues** — Les branches s'affichent avec des courbes fluides Unicode
- **Couleurs stables** — Chaque branche conserve sa couleur sur toute sa durée
- **Merges visibles** — Forks et merges représentés avec des courbes
- **Nœuds** : `●` commit normal, `○` merge commit, `◉` commit sélectionné
- **Infos** : hash (7 chars), branches/tags colorés, message, auteur, date relative

---

## Architecture du projet

```
src/
├── main.rs              # Point d'entrée, parsing CLI (clap)
├── app.rs               # Orchestration de l'application
├── error.rs             # Types d'erreurs custom (thiserror)
├── error_display.rs     # Formatage des messages d'erreur
├── terminal.rs          # Setup/teardown du terminal (raw mode)
├── watcher.rs           # Surveillance des changements git
│
├── git/                 # Couche d'abstraction git (git2)
│   ├── repo.rs          #   Wrapper GitRepo
│   ├── graph.rs         #   Algorithme de construction du graphe
│   ├── commit.rs        #   Opérations de commit
│   ├── branch.rs        #   Gestion des branches
│   ├── stash.rs         #   Opérations stash
│   ├── merge.rs         #   Opérations merge
│   ├── diff.rs          #   Génération de diffs
│   ├── blame.rs         #   Git blame
│   ├── conflict.rs      #   Détection et résolution de conflits
│   ├── remote.rs        #   Push, pull, fetch (SSH)
│   ├── search.rs        #   Recherche de commits
│   ├── worktree.rs      #   Gestion des worktrees
│   └── discard.rs       #   Discard des changements
│
├── handler/             # Gestionnaires d'événements
│   ├── mod.rs           #   Boucle événementielle (EventHandler)
│   ├── dispatcher.rs    #   Routage des actions
│   ├── navigation.rs    #   Navigation (scroll, sélection, focus)
│   ├── staging.rs       #   Stage, unstage, commit, discard
│   ├── branch.rs        #   Checkout, create, delete, rename
│   ├── conflict.rs      #   Résolution de conflits
│   ├── git.rs           #   Push, pull, fetch, cherry-pick
│   ├── search.rs        #   Recherche
│   ├── edit.rs          #   Édition de texte (prompts)
│   └── filter.rs        #   Filtres sur le graphe
│
├── state/               # État de l'application
│   ├── mod.rs           #   AppState (état central)
│   ├── action/          #   Enums d'actions (AppAction et sous-enums)
│   ├── view/            #   États spécifiques à chaque vue
│   ├── selection.rs     #   Sélection générique dans une liste
│   ├── cache.rs         #   Cache LRU pour les diffs
│   └── filter.rs        #   État des filtres
│
├── ui/                  # Rendu de l'interface
│   ├── mod.rs           #   Dispatcher de rendu par vue
│   ├── graph_view.rs    #   Rendu du graphe git
│   ├── detail_view.rs   #   Panneau détail d'un commit
│   ├── diff_view.rs     #   Rendu des diffs (unified/side-by-side)
│   ├── staging_view.rs  #   Vue staging
│   ├── branches_view.rs #   Vue branches/worktrees/stashes
│   ├── conflicts_view.rs#   Vue résolution de conflits
│   ├── blame_view.rs    #   Vue blame
│   ├── input.rs         #   Mapping des touches → actions
│   ├── theme.rs         #   Système de thème (dark/light)
│   ├── common/          #   Composants UI réutilisables
│   └── ...              #   Autres composants (popups, barres, etc.)
│
├── utils/               # Utilitaires
│   └── time.rs          #   Formatage de dates relatives
│
└── test_utils/          # Utilitaires de test
    ├── mock_repo.rs     #   Mock repository pour les tests
    ├── test_state.rs    #   Builder d'état de test
    └── assertions.rs    #   Macros d'assertions custom
```

Pour plus de détails, voir [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## Développement

### Prérequis

- **Rust** 2021 edition (1.56+)
- **libgit2** (fourni par la crate `git2`, ou utiliser `--features vendored-ssl`)
- **OpenSSL** (requis pour les opérations remote, sauf si `vendored-ssl`)

### Stack technique

| Crate            | Rôle                            |
| ---------------- | ------------------------------- |
| `ratatui` 0.29   | Framework TUI                   |
| `crossterm` 0.28 | Backend terminal cross-platform |
| `git2` 0.19      | Bindings libgit2                |
| `clap` 4         | Parsing CLI (derive)            |
| `anyhow`         | Gestion d'erreurs applicatives  |
| `thiserror`      | Types d'erreurs custom          |
| `chrono`         | Formatage des dates             |
| `arboard`        | Accès au presse-papiers         |
| `lru`            | Cache LRU pour les diffs        |
| `terminal-light` | Détection du thème terminal     |

### Commandes

```bash
# Build
cargo build
cargo build --release

# Exécuter
cargo run
cargo run -- --path /chemin/vers/repo
cargo run -- log -n 10

# Tests (127 tests)
cargo test
cargo test nom_du_test
cargo test module::          # Tests d'un module
cargo test -- --nocapture    # Avec la sortie visible

# Couverture (nécessite cargo-tarpaulin)
cargo tarpaulin --out Html --output-dir coverage

# Formatage
cargo fmt
cargo fmt -- --check

# Lint
cargo clippy
cargo clippy --all-features -- -D warnings

# Vérification rapide
cargo check
```

### Conventions de code

- **Imports** : `std` → crates externes → modules internes (`use crate::`)
- **Nommage** : `PascalCase` (types), `snake_case` (fonctions), `UPPER_SNAKE_CASE` (constantes)
- **Commentaires** : en français
- **Erreurs** : `crate::error::Result` dans les modules, `anyhow::Result` dans `main.rs`
- **Pattern matching** : exhaustif avec `_`, pas de chaînes `if/else`

### Checklist avant commit

1. `cargo build` réussit
2. `cargo test` passe (127 tests)
3. `cargo fmt` appliqué
4. `cargo clippy` sans warnings

---

## Licence

MIT — voir [LICENSE](LICENSE)
