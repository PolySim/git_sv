# Guide de contribution a git_sv

Merci de contribuer a `git_sv`.

Ce document resume la mise en place, les conventions du projet et les points d'entree utiles pour ajouter ou modifier une fonctionnalite sans se perdre dans l'architecture.

---

## Prerequis

- Rust stable recent
- git
- `libgit2` disponible sur la machine si vous ne compilez pas avec `--features vendored-ssl`

Exemples :

```bash
# macOS
brew install libgit2

# Debian / Ubuntu
apt install libgit2-dev

# Arch
pacman -S libgit2
```

Outils recommandes :

```bash
rustup component add rustfmt
rustup component add clippy
cargo install cargo-tarpaulin
```

---

## Installation locale

```bash
git clone <url-du-repo> git_sv
cd git_sv

cargo build
cargo run
```

Executer sur un autre repository :

```bash
cargo run -- --path /chemin/vers/repo
```

Compilation release :

```bash
cargo build --release
```

Si OpenSSL pose probleme :

```bash
cargo build --release --features vendored-ssl
```

---

## Commandes utiles

```bash
# Verification rapide
cargo check

# Formatage
cargo fmt
cargo fmt -- --check

# Lint
cargo clippy
cargo clippy --all-features -- -D warnings

# Tests
cargo test
cargo test nom_du_test
cargo test module::
cargo test -- --nocapture

# Integration seulement
cargo test --test integration_test
```

---

## Conventions de code

### Langue

- commentaires et doc comments en francais ;
- noms de types en `PascalCase` ;
- fonctions, variables et modules en `snake_case` ;
- constantes en `UPPER_SNAKE_CASE`.

### Imports

Ordre recommande :

1. bibliotheque standard ;
2. crates externes ;
3. modules internes via `crate::...`.

### Erreurs

- `anyhow::Result` au point d'entree ;
- `crate::error::Result` dans les modules internes ;
- utiliser `?` autant que possible ;
- ajouter du contexte quand cela clarifie l'echec.

### Tests

- tests unitaires dans le fichier concerne sous `#[cfg(test)]` ;
- tests d'integration dans `tests/` ;
- utiliser `tempfile` et les helpers existants de `src/test_utils/` quand c'est pertinent.

---

## Ajouter une nouvelle vue

### 1. Ajouter l'etat de vue

- creer le state dans `src/state/view/` ;
- re-exporter le type dans `src/state/view/mod.rs` ;
- ajouter le variant dans `ViewMode` si necessaire.

### 2. Ajouter les actions

- creer un enum dans `src/state/action/` ;
- l'ajouter a `AppAction` via `src/state/action/mod.rs`.

### 3. Ajouter le rendu

- creer le renderer dans `src/ui/` ;
- declarer le module dans `src/ui/mod.rs` ;
- brancher le rendu dans le `match` de `ui::render()`.

### 4. Ajouter la logique de handling

- creer un handler dans `src/handler/` ou `src/handler/<domaine>/` ;
- l'enregistrer dans `src/handler/dispatcher/mod.rs`.

### 5. Ajouter les raccourcis clavier/souris

- clavier : `src/ui/input/keyboard.rs` ;
- souris : `src/ui/input/mouse.rs` si necessaire ;
- tests associes : `src/ui/input/tests.rs`.

---

## Ajouter un nouveau comportement sans nouvelle vue

Selon la nature du changement :

- navigation : `src/handler/navigation.rs`
- git : `src/handler/git.rs`
- staging : `src/handler/staging.rs`
- branches/worktrees/stashes : `src/handler/branch.rs`
- recherche : `src/handler/search.rs`
- filtres : `src/handler/filter.rs`
- confirmations / pickers / clipboard : `src/handler/dispatcher/`

Dans la plupart des cas il faut :

1. ajouter ou etendre une action ;
2. brancher le mapping dans `src/ui/input/keyboard.rs` ;
3. traiter l'action dans le bon handler ou dans `src/handler/dispatcher/mod.rs`.

---

## Fichiers reperes

| Besoin | Fichier |
|--------|---------|
| Point d'entree | `src/main.rs` |
| CLI non interactif | `src/cli.rs` |
| Initialisation app | `src/app.rs` |
| Etat global | `src/state/mod.rs` |
| Actions | `src/state/action/` |
| Etats de vues | `src/state/view/` |
| Boucle evenementielle | `src/handler/mod.rs` |
| Dispatcher principal | `src/handler/dispatcher/mod.rs` |
| Mapping clavier | `src/ui/input/keyboard.rs` |
| Mapping souris | `src/ui/input/mouse.rs` |
| Rendu principal | `src/ui/mod.rs` |
| Watcher git | `src/watcher.rs` |

---

## Checklist avant PR

- `cargo build` passe
- `cargo test` passe
- `cargo fmt -- --check` passe
- `cargo clippy --all-features -- -D warnings` passe si vous touchez du code significatif
- la documentation est mise a jour si l'architecture, les commandes ou les raccourcis changent
- les commentaires ajoutes restent en francais

---

## Conseils pratiques

- preservez les conventions existantes avant de refactorer plus largement ;
- evitez d'etendre la couche legacy si une version plus recente existe deja ;
- si vous ajoutez une sortie CLI, pensez aux formats `human`, `plain` et `json` ;
- si vous modifiez le graphe ou la navigation, ajoutez au moins un test de non-regression.
