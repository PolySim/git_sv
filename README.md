# git_sv - Visualisez Git dans votre terminal

[![CI](https://github.com/PolySim/git_sv/actions/workflows/ci.yml/badge.svg)](https://github.com/PolySim/git_sv/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`git_sv` est un client Git en terminal ecrit en Rust.

Il combine :

- une TUI pour naviguer dans l'historique, les diffs, le staging et les branches ;
- un mode CLI pour les usages rapides et les scripts ;
- une lecture du graphe git plus visuelle que les commandes terminal classiques.

En une phrase : `git_sv` cherche a apporter une experience proche d'un client Git graphique, sans quitter le terminal.

![Rust](https://img.shields.io/badge/Rust-2021-orange) ![ratatui](https://img.shields.io/badge/TUI-ratatui-green)

---

## Pourquoi git_sv ?

- pour lire un historique git complexe sans se battre avec un log brut ;
- pour stager, committer et naviguer plus vite sans sortir du terminal ;
- pour garder un outil scriptable grace au mode CLI ;
- pour offrir une experience terminal plus moderne autour de Git.

## Points forts

- graphe de commits avec couleurs stables par branche ;
- staging interactif avec diff et message de commit ;
- gestion des branches locales et distantes ;
- worktrees et stashes ;
- recherche de commits et filtres sur le graphe ;
- blame et resolution de conflits ;
- operations distantes `push`, `pull`, `fetch` ;
- rafraichissement automatique quand l'etat git change.

---

## Installation

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

### Homebrew

Le fichier `homebrew/git_sv.rb` est present dans le depot, mais la formule n'est pas encore publiable tant que les `sha256` des archives de release ne sont pas renseignes.

### OpenSSL

Si votre systeme n'expose pas correctement OpenSSL, essayez :

```bash
cargo build --release --features vendored-ssl
```

---

## Utilisation

### Mode interactif

```bash
# Dans un repository git
git_sv

# Sur un repository specifique
git_sv --path /chemin/vers/repo
git_sv -p /chemin/vers/repo
```

### Mode CLI

```bash
# Historique
git_sv log
git_sv log -n 50
git_sv log --author "Alice"
git_sv log --message "fix"
git_sv log --since 2024-01-01 --until 2024-12-31

# Branches
git_sv branches
git_sv branches --format json

# Status
git_sv status
git_sv status --format plain

# Recherche
git_sv search "fix bug"

# Graphe simplifie
git_sv graph -n 30
git_sv graph --format json
```

### Formats de sortie CLI

- `human` : sortie lisible avec couleurs ;
- `plain` : texte simple ;
- `json` : sortie structuree pour scripts.

### Exemples

```bash
# Extraire les hashes d'un auteur
git_sv log --author "Alice" --format json | jq '.[].hash'

# Verifier rapidement les fichiers modifies
git_sv status --format plain | grep "^M"

# Trouver le dernier commit ayant touche un fichier
git_sv log -n 1 --path-filter src/main.rs
```

---

## Raccourcis clavier principaux

### Navigation globale

| Touche | Action |
|--------|--------|
| `1` | Vue graph |
| `2` | Vue staging |
| `3` | Vue branches |
| `4` | Vue conflits si active |
| `?` | Aide |
| `q` | Quitter |
| `Ctrl+c` | Quitter |

### Vue Graph

| Touche | Action |
|--------|--------|
| `j` / `k` | Naviguer dans les commits |
| `g` / `G` | Aller au debut / a la fin |
| `Ctrl+d` / `Ctrl+u` | Page suivante / precedente |
| `Tab` | Changer de panneau |
| `Enter` | Ouvrir le panneau fichiers depuis le graphe |
| `z` | Ouvrir ou fermer le diff plein ecran |
| `M` | Basculer le panneau bas-gauche |
| `r` | Rafraichir |
| `P` | Push |
| `Ctrl+p` | Force push |
| `p` | Pull |
| `f` | Fetch |
| `x` | Cherry-pick |
| `y` | Copier le contenu du panneau actif |
| `/` | Recherche |
| `F` | Filtres |
| `v` | Basculer le mode de diff |
| `L` | Charger plus d'historique |

### Vue Staging

| Touche | Action |
|--------|--------|
| `j` / `k` | Naviguer dans les fichiers |
| `s` | Stage fichier |
| `u` | Unstage fichier |
| `a` | Stage tout |
| `U` | Unstage tout |
| `d` | Discard fichier |
| `D` | Discard tout |
| `c` | Editer le message de commit |
| `A` | Amend |
| `Tab` | Changer de focus |

### Vue Branches

| Touche | Action |
|--------|--------|
| `j` / `k` | Naviguer |
| `Tab` / `Shift+Tab` | Changer de section |
| `Enter` | Checkout |
| `n` | Creer branche ou worktree |
| `d` | Supprimer l'element courant |
| `r` | Renommer la branche |
| `R` | Afficher ou masquer les branches distantes |
| `m` | Merge |
| `s` | Sauver un stash |
| `a` | Appliquer un stash |
| `p` | Pop un stash |

---

## Architecture

Structure actuelle simplifiee :

```text
src/
├── main.rs
├── app.rs
├── cli.rs
├── config.rs
├── error.rs
├── i18n.rs
├── terminal.rs
├── watcher.rs
├── git/
├── handler/
│   ├── dispatcher/
│   └── conflict/
├── state/
│   ├── action/
│   └── view/
├── test_utils/
└── ui/
    ├── common/
    └── input/
```

Pour le detail des modules et du flux d'execution, voir `docs/ARCHITECTURE.md`.

---

## Developpement

### Commandes de base

```bash
# Build
cargo build
cargo build --release

# Lancer
cargo run
cargo run -- --path /chemin/vers/repo
cargo run -- log -n 10

# Tests
cargo test
cargo test nom_du_test
cargo test module::
cargo test -- --nocapture

# Formatage
cargo fmt
cargo fmt -- --check

# Lint
cargo clippy
cargo clippy --all-features -- -D warnings

# Verification rapide
cargo check
```

### Conventions

- imports groupes : `std`, crates externes, modules internes ;
- commentaires en francais ;
- types en `PascalCase`, fonctions en `snake_case` ;
- `crate::error::Result` dans les modules, `anyhow::Result` au point d'entree.

---

## Pour communiquer sur le projet

- changelog : `CHANGELOG.md`
- kit de communication : `docs/COMMUNICATION.md`
- architecture : `docs/ARCHITECTURE.md`
- contribution : `docs/CONTRIBUTING.md`

---

## Licence

MIT - voir `LICENSE`.
