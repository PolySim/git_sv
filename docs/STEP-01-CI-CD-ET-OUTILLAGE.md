# STEP 01 — CI/CD et outillage de base

**Priorité** : Haute
**Effort estimé** : 1-2 jours
**Impact** : Fiabilité du projet, protection contre les régressions

---

## Constat

Le projet n'a **aucune CI sur les PR/push**. Le seul workflow GitHub Actions est `release.yml` qui se déclenche sur les tags `v*`. Cela signifie que des régressions peuvent être mergées sans être détectées.

Il manque également des fichiers de configuration essentiels pour un projet open-source.

---

## Actions à mener

### 1.1 — Créer un workflow CI (`ci.yml`)

Créer `.github/workflows/ci.yml` qui se déclenche sur `push` et `pull_request` :

```yaml
# Étapes requises :
- cargo fmt -- --check
- cargo clippy --all-features -- -D warnings
- cargo test
- cargo build
```

Matrice de build à considérer : `ubuntu-latest`, `macos-latest`, `windows-latest`.

### 1.2 — Ajouter un fichier LICENSE

Le `Cargo.toml` déclare `license = "MIT"` mais **aucun fichier LICENSE n'existe** dans le repository. C'est une obligation légale pour que la licence soit effectivement applicable.

- Créer `LICENSE` avec le texte standard MIT.

### 1.3 — Ajouter `rustfmt.toml`

Actuellement aucune configuration de formatage n'est définie. Ajouter un `rustfmt.toml` pour fixer les conventions :

```toml
edition = "2021"
max_width = 100
```

### 1.4 — Ajouter `.clippy.toml` ou configurer Clippy dans `Cargo.toml`

Définir les lints clippy au niveau du projet pour homogénéiser la qualité du code.

### 1.5 — Ajouter le badge CI dans le README

Une fois la CI en place, ajouter un badge de statut en haut du README.

### 1.6 — Vérifier le `release.yml` existant

Le workflow de release utilise des placeholders dans `homebrew/git_sv.rb` (`PLACEHOLDER` pour les SHA256). Vérifier que le workflow les remplace correctement.

---

## Critères de validation

- [ ] `cargo fmt -- --check` passe en CI
- [ ] `cargo clippy --all-features -- -D warnings` passe en CI
- [ ] `cargo test` passe en CI
- [ ] Fichier `LICENSE` présent
- [ ] Badge CI visible dans le README
- [ ] CI se déclenche automatiquement sur chaque push/PR
