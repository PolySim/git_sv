# git_sv - Interface Terminal Git

Un visualiseur git en terminal avec graphe de commits style GitKraken, staging interactif, et gestion des branches/worktrees/stashes.

## Installation

### Via Homebrew (recommandé)

```bash
brew tap PolySim/tap
brew install git_sv
```

Pour mettre à jour vers la dernière version :

```bash
brew upgrade git_sv
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

## Navigation entre les vues

| Touche   | Action                                      |
| -------- | ------------------------------------------- |
| `1`      | Vue Graph (historique des commits)          |
| `2`      | Vue Staging (staging et commits)            |
| `3`      | Vue Branches (branches, worktrees, stashes) |
| `?`      | Afficher/masquer l'aide                     |
| `q`      | Quitter                                     |
| `Ctrl+c` | Quitter (force)                             |

---

## Vue Graph (1)

Visualisation du graphe de commits avec affichage style GitKraken.

### Navigation

| Touche       | Action           |
| ------------ | ---------------- |
| `j` / `↓`    | Commit suivant   |
| `k` / `↑`    | Commit précédent |
| `g` / `Home` | Premier commit   |
| `G` / `End`  | Dernier commit   |
| `Ctrl+d`     | Page suivante    |
| `Ctrl+u`     | Page précédente  |

### Focus et panneaux

| Touche  | Action                                                 |
| ------- | ------------------------------------------------------ |
| `Tab`   | Changer de panneau (Graph → Fichiers → Détail → Graph) |
| `Enter` | Sélectionner / entrer dans un panneau                  |
| `Esc`   | Retour au panneau précédent                            |

### Quand le focus est sur "Fichiers"

| Touche    | Action                              |
| --------- | ----------------------------------- |
| `j` / `k` | Naviguer dans la liste des fichiers |
| `Enter`   | Voir le diff du fichier             |

### Quand le focus est sur "Détail" (diff)

| Touche              | Action                    |
| ------------------- | ------------------------- |
| `j` / `k`           | Scroller le diff          |
| `Ctrl+d` / `Ctrl+u` | Page down/up dans le diff |

### Actions Git

| Touche | Action                       |
| ------ | ---------------------------- |
| `b`    | Liste des branches (overlay) |
| `r`    | Rafraîchir                   |

### Dans l'overlay Branches

| Touche      | Action              |
| ----------- | ------------------- |
| `j` / `k`   | Naviguer            |
| `Enter`     | Checkout la branche |
| `n`         | Nouvelle branche    |
| `d`         | Supprimer branche   |
| `Esc` / `b` | Fermer              |

---

## Vue Staging (2)

Interface de staging et création de commits.

### Layout

```
┌──────────────┬──────────────┐
│  Unstaged    │              │
├──────────────┤     Diff     │
│   Staged     │              │
├──────────────┴──────────────┤
│    Message de commit        │
└─────────────────────────────┘
```

### Navigation entre panneaux

| Touche    | Action                                     |
| --------- | ------------------------------------------ |
| `Tab`     | Cycle: Unstaged → Staged → Diff → Unstaged |
| `1` / `3` | Aller à une autre vue                      |

### Dans "Unstaged" (fichiers non stagés)

| Touche        | Action                       |
| ------------- | ---------------------------- |
| `j` / `k`     | Naviguer dans les fichiers   |
| `s` / `Enter` | Stage le fichier sélectionné |
| `a`           | Stage tous les fichiers      |

### Dans "Staged" (fichiers stagés)

| Touche        | Action                         |
| ------------- | ------------------------------ |
| `j` / `k`     | Naviguer dans les fichiers     |
| `u` / `Enter` | Unstage le fichier sélectionné |
| `U`           | Unstage tous les fichiers      |

### Dans "Diff" (visualisation des changements)

| Touche        | Action            |
| ------------- | ----------------- |
| `j` / `k`     | Scroller le diff  |
| `Tab` / `Esc` | Retour à Unstaged |

### Créer un commit

| Touche      | Action                                                     |
| ----------- | ---------------------------------------------------------- |
| `c`         | Activer le champ de message                                |
| `Entrée`    | Valider le commit (si message non vide et fichiers stagés) |
| `Esc`       | Annuler la saisie                                          |
| `←` / `→`   | Déplacer le curseur                                        |
| `Backspace` | Supprimer un caractère                                     |

---

## Vue Branches (3)

Gestion des branches, worktrees et stashes avec 3 onglets.

### Navigation générale

| Touche      | Action                                            |
| ----------- | ------------------------------------------------- |
| `Tab`       | Section suivante (Branches → Worktrees → Stashes) |
| `Shift+Tab` | Section précédente                                |
| `1` / `2`   | Aller à une autre vue                             |

### Onglet Branches

#### Navigation

| Touche    | Action                           |
| --------- | -------------------------------- |
| `j` / `k` | Naviguer dans les branches       |
| `R`       | Toggle affichage branches remote |

#### Actions

| Touche  | Action                                       |
| ------- | -------------------------------------------- |
| `Enter` | Checkout la branche sélectionnée             |
| `n`     | Créer une nouvelle branche (ouvre un prompt) |
| `d`     | Supprimer la branche sélectionnée            |
| `r`     | Renommer la branche (ouvre un prompt)        |

**Note** : Impossible de supprimer la branche courante (HEAD).

### Onglet Worktrees

#### Navigation

| Touche    | Action                      |
| --------- | --------------------------- |
| `j` / `k` | Naviguer dans les worktrees |

#### Actions

| Touche | Action                                             |
| ------ | -------------------------------------------------- |
| `n`    | Créer un worktree (format: `nom chemin [branche]`) |
| `d`    | Supprimer le worktree sélectionné                  |

**Note** : Impossible de supprimer le worktree principal.

### Onglet Stashes

#### Navigation

| Touche    | Action                    |
| --------- | ------------------------- |
| `j` / `k` | Naviguer dans les stashes |

#### Actions

| Touche | Action                                                 |
| ------ | ------------------------------------------------------ |
| `a`    | Appliquer le stash (sans supprimer)                    |
| `p`    | Pop le stash (appliquer + supprimer)                   |
| `d`    | Supprimer le stash                                     |
| `s`    | Sauvegarder un stash (ouvre un prompt pour le message) |

### Dans un prompt d'input

| Touche      | Action                 |
| ----------- | ---------------------- |
| `Entrée`    | Confirmer              |
| `Esc`       | Annuler                |
| `←` / `→`   | Déplacer le curseur    |
| `Backspace` | Supprimer un caractère |

---

## Caractéristiques du graphe

- **Lignes continues** : Les branches s'affichent avec des lignes fluides (style GitKraken)
- **Couleurs stables** : Chaque branche garde sa couleur du début à la fin
- **Merges visibles** : Les merges et forks sont représentés avec des courbes
- **Nœuds** :
  - `●` Commit normal
  - `○` Merge commit
  - `◉` Commit sélectionné

## Infos affichées

Pour chaque commit :

- Hash (7 caractères)
- Branches et tags (labels colorés)
- Message de commit
- Auteur
- Date

Pour chaque fichier modifié :

- Statut (Added, Modified, Deleted, Renamed)
- Nombre de lignes ajoutées/supprimées

## Configuration

Le programme détecte automatiquement le repository git dans le répertoire courant ou les répertoires parents.

## Développement

### Stack technique

- **Langage** : Rust
- **TUI Framework** : ratatui
- **Terminal Backend** : crossterm
- **Git** : git2
- **CLI** : clap (derive)
- **Gestion d'erreurs** : anyhow, thiserror
- **Dates** : chrono

### Commandes utiles

```bash
# Build
cargo build

# Build release (optimisé)
cargo build --release

# Exécuter
cargo run
cargo run -- log -n 10

# Tests (35 tests)
cargo test
cargo test module_name::
cargo test -- --nocapture

# Formatage
cargo fmt
cargo fmt -- --check

# Lint et vérifications
cargo clippy
cargo clippy --all-features -- -D warnings
cargo check
```

### Convention de code

- **Imports** : std → crates externes → modules internes
- **Noms** : Types (PascalCase), fonctions (snake_case), constantes (UPPER_SNAKE_CASE)
- **Commentaires** : Français pour les commentaires inline
- **Documentation** : Anglais pour les doc comments (`///`)
- **Gestion d'erreurs** : Utiliser `anyhow::Result` ou `crate::error::Result`
- **Pattern matching** : Exhaustif avec `_` plutôt que `if/else`

### Checklist avant de commiter

1. ✅ `cargo build` réussit
2. ✅ `cargo test` passe (tous les 35 tests)
3. ✅ `cargo fmt` appliqué
4. ✅ `cargo clippy` sans warnings
5. ✅ Code suit les conventions de nommage

## Licence

MIT
