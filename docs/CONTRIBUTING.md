# Guide de contribution à git_sv

Merci de l'intérêt porté à `git_sv` ! Ce guide explique comment mettre en place l'environnement de développement, contribuer du code et suivre les conventions du projet.

---

## Table des matières

1. [Prérequis](#prérequis)
2. [Installation et setup](#installation-et-setup)
3. [Lancer les tests](#lancer-les-tests)
4. [Conventions de code](#conventions-de-code)
5. [Ajouter une nouvelle vue](#ajouter-une-nouvelle-vue)
6. [Ajouter un nouveau handler](#ajouter-un-nouveau-handler)
7. [Processus de contribution (PR)](#processus-de-contribution-pr)
8. [Checklist avant commit](#checklist-avant-commit)

---

## Prérequis

- **Rust** stable (≥ 1.75) — [https://rustup.rs](https://rustup.rs)
- **libgit2** (dépendance système pour la crate `git2`)
  - macOS : `brew install libgit2`
  - Debian/Ubuntu : `apt install libgit2-dev`
  - Arch : `pacman -S libgit2`
- **git** (pour les tests d'intégration)

### Outils recommandés

```bash
# Formatter
rustup component add rustfmt

# Linter
rustup component add clippy

# Couverture (optionnel)
cargo install cargo-tarpaulin
```

---

## Installation et setup

```bash
# Cloner le repository
git clone <url-du-repo> git_sv
cd git_sv

# Compiler en mode développement
cargo build

# Lancer l'application sur le repo courant
cargo run

# Lancer sur un autre repository
cargo run -- --path /chemin/vers/repo

# Compiler en mode release (optimisé)
cargo build --release
```

---

## Lancer les tests

```bash
# Tous les tests
cargo test

# Tests d'un module spécifique
cargo test git::
cargo test handler::
cargo test state::

# Tests avec output visible (pour debug)
cargo test -- --nocapture

# Tests correspondant à un pattern
cargo test test_dispatch

# Tests d'intégration uniquement
cargo test --test integration_test

# Couverture HTML (nécessite cargo-tarpaulin)
cargo tarpaulin --out Html --output-dir coverage
# Ouvrir coverage/tarpaulin-report.html
```

---

## Conventions de code

### Langue

| Contexte                   | Langue      |
|----------------------------|-------------|
| Commentaires inline (`//`) | Français    |
| Doc comments (`///`, `//!`)| Français    |
| Noms de variables/fonctions| Anglais (snake_case) |
| Messages d'erreur UI       | Français    |
| Messages de commit git     | Français    |

### Nommage

```rust
// Types : PascalCase
struct AppState { ... }
enum ViewMode { Graph, Staging }

// Fonctions et variables : snake_case
fn build_graph() -> Vec<GraphRow> { ... }
let selected_index = 0;

// Constantes : UPPER_SNAKE_CASE
const MAX_COMMITS: usize = 200;

// Modules : snake_case
mod graph_view;
mod test_utils;
```

### Gestion d'erreurs

```rust
// Utiliser anyhow::Result pour les fonctions applicatives
use crate::error::Result;

// Propager avec ?
fn open_repo(path: &str) -> Result<GitRepo> {
    let repo = git2::Repository::open(path)?;
    Ok(GitRepo { repo })
}

// Ajouter du contexte si utile
use crate::error::IoErrorContext;
let file = File::open(path).with_context(|| format!("Ouverture de {}", path))?;
```

### Imports

```rust
// 1. Bibliothèque standard
use std::io::{self, Stdout};

// 2. Crates externes
use ratatui::{backend::CrosstermBackend, Terminal};
use anyhow::Result;

// 3. Modules internes
use crate::error::Result;
use crate::git::GitRepo;
```

### Structure d'un module

Chaque fichier doit commencer par un commentaire modulaire `//!` :

```rust
//! Description courte du module.
//!
//! Détails supplémentaires si nécessaire.

use ...
```

### Tests

Les tests unitaires se placent dans le même fichier, sous `#[cfg(test)]` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mock_repo::create_test_repo;

    #[test]
    fn test_ma_fonctionnalite() {
        // Arrange
        let repo = create_test_repo();
        // Act
        let result = ma_fonction(&repo);
        // Assert
        assert_eq!(result, valeur_attendue);
    }
}
```

---

## Ajouter une nouvelle vue

Une vue correspond à un `ViewMode`. Pour en ajouter une :

### 1. Ajouter le variant dans `state/view/mod.rs`

```rust
pub enum ViewMode {
    Graph,
    Staging,
    // ...
    MaNouvelle Vue, // <-- ici
}
```

### 2. Créer le fichier d'état dans `state/view/`

```rust
// src/state/view/ma_vue.rs
//! État de la nouvelle vue.

/// État de la vue MaNouvelle.
#[derive(Debug, Clone, Default)]
pub struct MaVueState {
    // champs...
}
```

Re-exporter dans `state/view/mod.rs`.

### 3. Créer le fichier de rendu dans `ui/`

```rust
// src/ui/ma_vue.rs
//! Rendu de la nouvelle vue.

use crate::state::MaVueState;
use ratatui::Frame;

pub fn render(frame: &mut Frame, state: &MaVueState, ...) {
    // ...
}
```

Déclarer dans `ui/mod.rs` et appeler depuis `ui::render()` dans le `match state.view_mode`.

### 4. Créer les actions dans `state/action/`

```rust
// src/state/action/ma_vue.rs
//! Actions de la nouvelle vue.

#[derive(Debug, Clone, PartialEq)]
pub enum MaVueAction {
    ActionA,
    ActionB(String),
}
```

Ajouter `MaVue(MaVueAction)` dans `AppAction`, re-exporter dans `state/action/mod.rs`.

### 5. Créer le handler dans `handler/`

```rust
// src/handler/ma_vue.rs
//! Handler pour la nouvelle vue.

use super::traits::{ActionHandler, HandlerContext};
use crate::state::action::MaVueAction;
use crate::error::Result;

pub struct MaVueHandler;

impl ActionHandler for MaVueHandler {
    type Action = MaVueAction;

    fn handle(&mut self, ctx: &mut HandlerContext, action: Self::Action) -> Result<()> {
        match action {
            MaVueAction::ActionA => { /* ... */ Ok(()) }
            MaVueAction::ActionB(data) => { /* ... */ Ok(()) }
        }
    }
}
```

Ajouter dans `handler/dispatcher.rs`.

### 6. Ajouter les keybindings dans `ui/input.rs`

Ajouter une fonction `map_ma_vue_key()` et l'appeler depuis `map_key()`.

---

## Ajouter un nouveau handler

Si vous souhaitez ajouter un handler sans nouvelle vue (ex : un nouveau type d'action git) :

1. Créer `src/state/action/mon_action.rs` avec l'enum d'action
2. L'ajouter dans `AppAction` et `state/action/mod.rs`
3. Créer `src/handler/mon_handler.rs` implémentant `ActionHandler`
4. L'enregistrer dans `ActionDispatcher::new()` et `dispatch()`
5. Mapper les touches dans `ui/input.rs`

---

## Processus de contribution (PR)

1. **Fork** le repository et créer une branche descriptive :
   ```bash
   git checkout -b feat/ma-fonctionnalite
   # ou
   git checkout -b fix/correction-bug
   ```

2. **Développer** en suivant les conventions ci-dessus.

3. **Tester** avant de pousser :
   ```bash
   cargo build
   cargo test
   cargo fmt
   cargo clippy
   ```

4. **Commit** avec un message clair en français :
   ```
   feat : ajouter le support des tags dans le graphe
   fix : corriger la navigation dans la vue blame
   refactor : simplifier le dispatcher d'actions
   docs : mettre à jour l'architecture
   test : ajouter les tests de navigation
   ```

5. **Ouvrir une Pull Request** avec :
   - Une description de la fonctionnalité ou du bug corrigé
   - Les tests ajoutés ou modifiés
   - Des captures d'écran si des changements UI sont impliqués

---

## Checklist avant commit

- [ ] `cargo build` réussit sans erreurs
- [ ] `cargo test` passe (tous les tests verts)
- [ ] `cargo fmt` appliqué (pas de diff de formatage)
- [ ] `cargo clippy` sans warnings
- [ ] Commentaires en français
- [ ] Doc comments `///` ajoutés aux types/fonctions publics
- [ ] `//!` en tête des nouveaux fichiers
- [ ] Tests ajoutés pour les nouvelles fonctionnalités
- [ ] `ARCHITECTURE.md` mis à jour si la structure a changé
