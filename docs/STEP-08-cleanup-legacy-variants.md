# STEP-08 : Nettoyage des variants legacy (FocusPanel, BottomLeftMode)

## Priorité : BASSE (dette technique)

## Problème

Le code contient des variants legacy dupliqués dans `FocusPanel` et `BottomLeftMode` qui créent de la confusion et des risques de bugs. Chaque `match` doit gérer les deux formes, et des incohérences sont possibles si une branche oublie un variant.

### Code actuel

```rust
// src/state/view/mod.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusPanel {
    #[default]
    Graph,
    BottomLeft,
    BottomRight,
    /// Legacy: équivalent à BottomLeft
    Files,
    /// Legacy: équivalent à BottomRight
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BottomLeftMode {
    #[default]
    Files,
    Parents,
    /// Legacy: équivalent à Files
    CommitFiles,
    /// Legacy: équivalent à Parents
    WorkingDir,
}
```

### Problèmes concrets

1. **`FocusPanel::Files` vs `FocusPanel::BottomLeft`** — Les deux sont utilisés dans le code :
   - `input.rs` utilise `FocusPanel::Files` et `FocusPanel::Detail` dans les match arms
   - `navigation.rs` utilise `FocusPanel::BottomLeft` et `FocusPanel::BottomRight`
   - `mod.rs` (UI) utilise `matches!(state.focus, FocusPanel::Files | FocusPanel::BottomLeft)`

2. **Match non exhaustifs** — Des `match` sur `FocusPanel` doivent lister les 5 variants, avec des branches legacy dupliquées :
   ```rust
   // src/handler/navigation.rs
   state.focus = match state.focus {
       FocusPanel::Graph => FocusPanel::BottomLeft,
       FocusPanel::BottomLeft => FocusPanel::BottomRight,
       FocusPanel::BottomRight => FocusPanel::Graph,
       // Legacy
       FocusPanel::Files => FocusPanel::BottomRight,
       FocusPanel::Detail => FocusPanel::Graph,
   };
   ```

3. **Risque de bugs** — Si un nouveau handler utilise `FocusPanel::Files` au lieu de `FocusPanel::BottomLeft`, le comportement peut être incohérent.

## Fichiers impactés

| Fichier | Impact |
|---------|--------|
| `src/state/view/mod.rs` | Suppression des variants legacy |
| `src/ui/input.rs` | Remplacement `Files` → `BottomLeft`, `Detail` → `BottomRight` |
| `src/ui/mod.rs` | Simplification des `matches!` |
| `src/handler/navigation.rs` | Suppression des branches legacy |
| `src/handler/dispatcher.rs` | Vérification des usages |
| `src/state/mod.rs` | Vérification des usages |
| `src/ui/files_view.rs` | Vérification des usages de `BottomLeftMode` |
| `src/ui/help_bar.rs` | Vérification |

## Solution proposée

### Étape 1 : Rechercher tous les usages

```bash
# Trouver toutes les occurrences des variants legacy
grep -rn "FocusPanel::Files\b" src/
grep -rn "FocusPanel::Detail\b" src/
grep -rn "BottomLeftMode::CommitFiles\b" src/
grep -rn "BottomLeftMode::WorkingDir\b" src/
```

### Étape 2 : Remplacer les usages

Pour chaque occurrence :
- `FocusPanel::Files` → `FocusPanel::BottomLeft`
- `FocusPanel::Detail` → `FocusPanel::BottomRight`
- `BottomLeftMode::CommitFiles` → `BottomLeftMode::Files`
- `BottomLeftMode::WorkingDir` → `BottomLeftMode::Parents`

### Étape 3 : Supprimer les variants legacy

```rust
// src/state/view/mod.rs — APRÈS nettoyage

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusPanel {
    #[default]
    Graph,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BottomLeftMode {
    #[default]
    Files,
    Parents,
}

impl BottomLeftMode {
    pub fn toggle(&mut self) {
        *self = match self {
            BottomLeftMode::Files => BottomLeftMode::Parents,
            BottomLeftMode::Parents => BottomLeftMode::Files,
        };
    }

    pub fn is_commit_files(&self) -> bool {
        matches!(self, BottomLeftMode::Files)
    }

    pub fn is_working_dir(&self) -> bool {
        matches!(self, BottomLeftMode::Parents)
    }
}
```

### Étape 4 : Simplifier les match arms

```rust
// src/handler/navigation.rs — AVANT
state.focus = match state.focus {
    FocusPanel::Graph => FocusPanel::BottomLeft,
    FocusPanel::BottomLeft => FocusPanel::BottomRight,
    FocusPanel::BottomRight => FocusPanel::Graph,
    FocusPanel::Files => FocusPanel::BottomRight,   // ← Supprimé
    FocusPanel::Detail => FocusPanel::Graph,         // ← Supprimé
};

// src/handler/navigation.rs — APRÈS
state.focus = match state.focus {
    FocusPanel::Graph => FocusPanel::BottomLeft,
    FocusPanel::BottomLeft => FocusPanel::BottomRight,
    FocusPanel::BottomRight => FocusPanel::Graph,
};
```

```rust
// src/ui/mod.rs — AVANT
let is_files_focused = matches!(state.focus, FocusPanel::Files | FocusPanel::BottomLeft);

// src/ui/mod.rs — APRÈS
let is_files_focused = state.focus == FocusPanel::BottomLeft;
```

### Étape 5 : Supprimer les re-exports legacy

```rust
// src/state/mod.rs — supprimer si présent :
// pub use action::{BranchAction, ConflictAction, ...};
// Ces re-exports sont déjà gérés par `pub use view::*`
```

## Risques

- **Aucun risque fonctionnel** si tous les usages sont migrés.
- Le compilateur Rust **signalera toute occurrence manquée** grâce aux match exhaustifs.
- Compiler avec `cargo build` après les suppressions pour vérifier.

## Ordre des opérations

1. `cargo build` initial pour s'assurer que tout compile.
2. Remplacer tous les usages (grep + sed ou manuellement).
3. Supprimer les variants legacy.
4. `cargo build` — corriger les erreurs.
5. `cargo test` — vérifier que les tests passent.
6. `cargo clippy` — vérifier qu'il n'y a pas de warnings.

## Tests

Pas de tests spécifiques à ajouter — les tests existants couvrent déjà le comportement fonctionnel. La suppression des variants legacy est purement structurelle.

## Critère de validation

- Plus aucune occurrence de `FocusPanel::Files`, `FocusPanel::Detail`, `BottomLeftMode::CommitFiles`, `BottomLeftMode::WorkingDir` dans le code.
- `cargo build` compile.
- `cargo test` passe.
- `cargo clippy` ne lève pas de warnings.
- Les match arms sont simplifiés (3 variants au lieu de 5).
