# STEP 3 - Afficher les noms de branches au lieu de "Ours" / "Theirs"

## Problème

Les panneaux affichent `"Ours (HEAD)"` et `"Theirs"` en titres. L'utilisateur ne sait pas immédiatement quelle branche correspond à quel côté. Il faudrait afficher les vrais noms de branches, par exemple `"main (HEAD)"` et `"feature/login"`.

## Contexte actuel

- `conflicts_view::render()` reçoit `current_branch: &Option<String>` (ligne 18), qui contient le nom de la branche locale (ours).
- `ConflictsState.operation_description` (ligne 413 de `state.rs`) contient une chaîne comme `"Merge de 'feature/login' dans 'main'"` construite dans `execute_merge()` (ligne ~2201 de `event.rs`).
- Le nom de la branche mergée (theirs) n'est stocké nulle part de façon structurée.
- Les titres `"Ours (HEAD)"` et `"Theirs"` sont hardcodés dans `render_ours_theirs_panels()` (lignes ~185-193 de `conflicts_view.rs`).

## Fichiers à modifier

| Fichier | Rôle |
|---------|------|
| `src/state.rs` | Ajouter `ours_branch` et `theirs_branch` dans `ConflictsState` |
| `src/event.rs` | Passer les noms de branches lors de la création du `ConflictsState` |
| `src/ui/conflicts_view.rs` | Utiliser les noms de branches dans les titres des panneaux |

## Modifications détaillées

### 1. `src/state.rs` — `ConflictsState`

Ajouter deux champs :

```rust
pub struct ConflictsState {
    // ... champs existants ...
    /// Nom de la branche "ours" (HEAD).
    pub ours_branch: String,
    /// Nom de la branche "theirs" (branche mergée).
    pub theirs_branch: String,
}
```

Modifier `ConflictsState::new()` pour accepter ces paramètres :

```rust
pub fn new(
    files: Vec<ConflictFile>,
    operation_description: String,
    ours_branch: String,
    theirs_branch: String,
) -> Self {
    // ...
    Self {
        // ... champs existants ...
        ours_branch,
        theirs_branch,
    }
}
```

### 2. `src/event.rs` — Tous les appels à `ConflictsState::new()`

Il y a 3 sites d'appel :

**a) `execute_merge()` (ligne ~2196)** — merge classique :
```rust
ConflictsState::new(
    files,
    format!("Merge de '{}' dans '{}'", branch_name, current_branch),
    current_branch.clone(),    // ours
    branch_name.to_string(),   // theirs
)
```

**b) Pull avec conflits (ligne ~1664)** — pull depuis origin :
```rust
ConflictsState::new(
    files,
    "Pull depuis origin".into(),
    current_branch.clone(),       // ours
    format!("origin/{}", current_branch),  // theirs
)
```

**c) Rebase/cherry-pick avec conflits (ligne ~2042)** si applicable :
Vérifier tous les appels et passer les noms de branches appropriés.

### 3. `src/ui/conflicts_view.rs` — Titres des panneaux

Modifier `render_ours_theirs_panels()` (ligne ~165) :

Remplacer :
```rust
.title("Ours (HEAD)")
// ...
.title("Theirs")
```

Par :
```rust
.title(format!("{} (ours)", state.ours_branch))
// ...
.title(format!("{} (theirs)", state.theirs_branch))
```

Mettre aussi à jour la help bar et le help overlay pour refléter que les noms sont dynamiques.

## Résultat attendu

- Le panneau gauche affiche `"main (ours)"` au lieu de `"Ours (HEAD)"`.
- Le panneau droit affiche `"feature/login (theirs)"` au lieu de `"Theirs"`.
- Lors d'un pull, le panneau droit affiche `"origin/main (theirs)"`.
