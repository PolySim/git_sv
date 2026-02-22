# STEP-5 : Le raccourci `s` (stash) ne fait rien depuis la vue Graph

## Problème

Presser `s` en vue Graph devrait créer un stash rapide ou ouvrir un prompt pour le message du stash, mais rien ne se passe.

## Cause racine

`handle_stash_prompt()` dans `handler/git.rs` est un **stub vide**.

### Flux cassé

1. **`input.rs:222`** mappe `s` → `AppAction::StashPrompt`
2. **`dispatcher.rs:173`** route vers `GitHandler` → `handle_stash_prompt()`
3. **`handler/git.rs:~209-211`** :
   ```rust
   fn handle_stash_prompt(state: &mut AppState) -> Result<()> {
       // Ouvre le prompt de stash (affichage UI)
       Ok(())
   }
   ```
   **Stub vide.**

## Fichiers concernés

| Fichier | Ligne(s) | Problème |
|---|---|---|
| `src/handler/git.rs` | `handle_stash_prompt()` ~209-211 | Stub vide |

## Correction proposée

### Option A : Stash rapide (sans message)

Le plus simple — créer un stash immédiatement sans demander de message :

```rust
fn handle_stash_prompt(state: &mut AppState) -> Result<()> {
    match crate::git::stash::create_stash(&mut state.repo.repo, None) {
        Ok(_) => {
            state.set_flash_message("Stash créé ✓".to_string());
            state.mark_dirty();
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur stash: {}", e));
        }
    }
    Ok(())
}
```

> **Note :** Vérifier la signature de `crate::git::stash::create_stash()`. Si elle prend un message obligatoire, passer `"WIP"` ou un message par défaut.

### Option B : Basculer vers la vue Branches, section Stashes, en mode Input

Pour une UX plus riche permettant de taper un message :

```rust
fn handle_stash_prompt(state: &mut AppState) -> Result<()> {
    use crate::state::{BranchesSection, BranchesFocus};
    use crate::state::view::branches::InputAction;

    // Basculer en vue Branches, section Stashes, avec l'overlay d'input
    state.view_mode = ViewMode::Branches;
    state.branches_view_state.section = BranchesSection::Stashes;
    state.branches_view_state.focus = BranchesFocus::Input;
    state.branches_view_state.input_action = Some(InputAction::SaveStash);
    state.branches_view_state.input_text.clear();
    state.branches_view_state.input_cursor = 0;
    state.mark_dirty();
    Ok(())
}
```

> **Attention :** L'option B dépend de la bonne implémentation du handler d'input pour `InputAction::SaveStash` (qui est actuellement un stub dans `handler/branch.rs` → `handle_stash_save()`). Il faudra aussi implémenter `handle_stash_save()` et le `ConfirmInput` pour le cas `SaveStash`. Voir STEP-6.

### Recommandation

Commencer par l'**Option A** pour restaurer la fonctionnalité de base, puis évoluer vers l'Option B en STEP-6 quand les handlers d'input seront implémentés.

## Vérification

1. `cargo build` compile
2. Avoir des modifications non commitées dans le repo
3. Presser `s` en vue Graph → un stash est créé, message flash "Stash créé ✓"
4. Aller en vue Branches → section Stashes → le stash apparaît dans la liste
