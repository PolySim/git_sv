# Architecture initiale de git_sv

## Objectif

`git_sv` est un outil CLI interactif (TUI) en Rust permettant de visualiser le graphe git
d'un repository directement dans le terminal, et d'effectuer des opérations git courantes
(commit, merge, stash, branches, etc.).

---

## Stack technique

| Crate       | Version | Rôle                                      |
|-------------|---------|-------------------------------------------|
| ratatui     | 0.29    | Framework TUI (rendu terminal)            |
| crossterm   | 0.28    | Backend terminal (compatible macOS)       |
| git2        | 0.19    | Bindings libgit2 (opérations git)         |
| clap        | 4       | Parsing des arguments CLI (derive)        |
| anyhow      | 1       | Gestion d'erreurs applicatives            |
| thiserror   | 2       | Erreurs typées custom                     |
| chrono      | 0.4     | Formatage des dates de commits            |

---

## Structure du projet

```
git_sv/
├── Cargo.toml
├── .gitignore
├── docs/
│   └── ARCHITECTURE.md    # Ce fichier
└── src/
    ├── main.rs            # Point d'entrée, parsing CLI (clap), lancement app
    ├── app.rs             # Boucle événementielle, état global de l'application
    ├── error.rs           # Types d'erreurs custom (thiserror)
    ├── git/
    │   ├── mod.rs         # Re-exports du module git
    │   ├── repo.rs        # Wrapper autour de git2::Repository
    │   ├── graph.rs       # Construction du graphe de commits
    │   ├── commit.rs      # Opérations commit (create, amend, log)
    │   ├── branch.rs      # Opérations branches (list, create, checkout, delete)
    │   ├── stash.rs       # Opérations stash (list, save, pop, drop)
    │   └── merge.rs       # Opérations merge
    └── ui/
        ├── mod.rs         # Re-exports du module UI
        ├── graph_view.rs  # Rendu du graphe git (lignes, couleurs, branches)
        ├── status_view.rs # Panneau status (fichiers modifiés, staged, untracked)
        ├── detail_view.rs # Panneau détail d'un commit sélectionné
        ├── input.rs       # Gestion des keybindings et événements clavier
        └── layout.rs      # Disposition des panneaux (split horizontal/vertical)
```

---

## Étapes d'implémentation

### Étape 1 — Initialiser le projet Cargo

- Créer `Cargo.toml` avec les dépendances listées dans la stack technique.
- Configurer le nom du binaire (`git_sv`), l'édition Rust 2021.

### Étape 2 — Créer le .gitignore

- Ignorer `target/`, `.DS_Store` et les fichiers temporaires courants.

### Étape 3 — Créer le module d'erreurs (`src/error.rs`)

- Définir un enum `GitSvError` avec `thiserror` couvrant :
  - Erreurs git (`git2::Error`)
  - Erreurs I/O (`std::io::Error`)
  - Erreurs terminal/UI
- Définir un alias `pub type Result<T> = std::result::Result<T, GitSvError>`.

### Étape 4 — Créer le module git (`src/git/`)

- **`repo.rs`** : Struct `GitRepo` wrappant `git2::Repository`.
  Méthodes : `open()`, `log()`, `status()`, `branches()`, `stashes()`, `current_branch()`.
- **`graph.rs`** : Struct `CommitNode { oid, message, author, date, parents, branch_refs, column }`.
  Algorithme de placement en colonnes pour le graphe (style `git log --graph`).
  Gestion des lignes de connexion entre parents/enfants.
- **`commit.rs`** : Fonctions pour créer un commit, amender, lister le log.
- **`branch.rs`** : Fonctions pour lister, créer, checkout, supprimer des branches.
- **`stash.rs`** : Fonctions pour lister, sauvegarder, pop, drop des stashes.
- **`merge.rs`** : Fonctions pour lancer un merge entre branches.
- **`mod.rs`** : Re-exporte les sous-modules publics.

### Étape 5 — Créer le module UI (`src/ui/`)

- **`graph_view.rs`** : Rendu du graphe avec caractères Unicode (lignes verticales, diagonales, noeuds).
  Couleurs par branche (cycle de couleurs). Scroll vertical, sélection d'un commit avec flèches haut/bas.
- **`status_view.rs`** : Panneau affichant les fichiers modifiés, staged, untracked.
- **`detail_view.rs`** : Panneau affichant le détail d'un commit sélectionné (hash, auteur, date, message, diff).
- **`input.rs`** : Table de keybindings :
  - `q` : quitter
  - `j/k` ou flèches : navigation
  - `Enter` : détail du commit
  - `c` : commit
  - `s` : stash
  - `m` : merge
  - `b` : branches
  - `?` : aide
- **`layout.rs`** : Disposition des panneaux (split horizontal/vertical avec ratatui Layout).
- **`mod.rs`** : Re-exporte les sous-modules et expose `render()`.

### Étape 6 — Créer l'état applicatif (`src/app.rs`)

- Struct `App` contenant :
  - Le repo git (`GitRepo`)
  - La liste de commits / noeuds du graphe
  - L'index de sélection courante
  - Le mode actif (Graph, Status, Detail)
- Boucle `run()` :
  1. Poll des événements crossterm
  2. Dispatch vers `input.rs`
  3. Mise à jour de l'état
  4. Rendu UI via `Terminal::draw(|frame| ui::render(frame, &app))`

### Étape 7 — Créer le point d'entrée (`src/main.rs`)

- Définir les sous-commandes clap :
  - Mode par défaut (sans argument) : lance la TUI interactive
  - Potentiellement `log`, `commit`, etc. pour un usage non-interactif
- Ouvrir le repo git courant (`GitRepo::open(".")`)
- Initialiser le terminal crossterm et lancer `App::run()`

### Étape 8 — Vérifier la compilation

- Exécuter `cargo build` pour s'assurer que tout compile sans erreur.
- Corriger les éventuels problèmes.

---

## Flux de données

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────┐
│  main.rs    │────>│   app.rs         │────>│  ui/        │
│  (CLI/clap) │     │  (Event Loop)    │     │  (Renderer) │
└─────────────┘     └──────┬───────────┘     └──────┬──────┘
                           │                        │
                           v                        v
                    ┌──────────────┐         ┌─────────────────┐
                    │  git/        │         │ graph_view.rs   │
                    │  (GitRepo)   │         │ status_view.rs  │
                    └──────┬───────┘         │ detail_view.rs  │
                           │                 └─────────────────┘
              ┌────────────┼────────────┐
              v            v            v
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ graph.rs │ │commit.rs │ │ stash.rs │
        │          │ │branch.rs │ │ merge.rs │
        └──────────┘ └──────────┘ └──────────┘
```

- **main.rs** parse les arguments et lance la boucle applicative.
- **app.rs** orchestre tout : il lit les données git, gère les événements clavier, et demande à l'UI de se redessiner.
- **git/** fournit une couche d'abstraction propre au-dessus de libgit2.
- **ui/** se charge uniquement du rendu à partir de l'état fourni par `App`.

---

## Keybindings prévus

| Touche      | Action                          |
|-------------|---------------------------------|
| `q`         | Quitter                         |
| `j` / `↓`  | Sélection suivante              |
| `k` / `↑`  | Sélection précédente            |
| `Enter`     | Voir le détail du commit        |
| `c`         | Nouveau commit                  |
| `s`         | Stash                           |
| `m`         | Merge                           |
| `b`         | Liste des branches              |
| `?`         | Afficher l'aide                 |
