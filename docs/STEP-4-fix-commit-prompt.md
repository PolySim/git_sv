# STEP-4 : Le raccourci `c` (commit) ne fait rien depuis la vue Graph

## Problème

Presser `c` en vue Graph devrait ouvrir le prompt de commit (basculer en vue Staging avec le focus sur le message de commit), mais rien ne se passe.

## Cause racine

`handle_commit_prompt()` dans `handler/git.rs` est un **stub vide**.

### Flux cassé

1. **`input.rs:220`** mappe `c` → `AppAction::CommitPrompt`
2. **`dispatcher.rs:172`** route vers `GitHandler` → `handle_commit_prompt()`
3. **`handler/git.rs:~205-208`** :
   ```rust
   fn handle_commit_prompt(state: &mut AppState) -> Result<()> {
       // Ouvre le prompt de commit (affichage UI - pas d'opération directe)
       // L'UI s'occupera d'afficher le dialogue
       Ok(())
   }
   ```
   **Stub vide.** Le commentaire est erroné — il n'y a aucune logique UI qui prend le relais.

## Fichiers concernés

| Fichier | Ligne(s) | Problème |
|---|---|---|
| `src/handler/git.rs` | `handle_commit_prompt()` ~205-208 | Stub vide |

## Correction proposée

### Implémenter `handle_commit_prompt()` dans `handler/git.rs`

Le comportement attendu est de basculer en vue Staging et activer le mode commit :

```rust
fn handle_commit_prompt(state: &mut AppState) -> Result<()> {
    // Basculer en vue Staging avec le focus sur le message de commit
    state.view_mode = ViewMode::Staging;
    state.staging_state.is_committing = true;
    state.staging_state.focus = StagingFocus::CommitMessage;
    state.mark_dirty();
    Ok(())
}
```

Ajouter l'import `StagingFocus` si nécessaire en haut du fichier :
```rust
use crate::state::{AppState, ViewMode, FocusPanel, BlameState, StagingFocus};
```

## Vérification

1. `cargo build` compile
2. Lancer l'app en vue Graph, presser `c` → bascule en vue Staging avec le curseur dans le champ de message de commit
3. Taper un message, presser `Enter` → le commit est créé (si des fichiers sont stagés)
4. Presser `Esc` → annule le commit et revient au mode normal de Staging
