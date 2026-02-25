# STEP-003 : Vue Branch — Enter ne fait plus le checkout de branche

## Problème

Dans la vue Branches (`ViewMode::Branches`), appuyer sur Enter ne fait rien.
Le checkout de branche ne fonctionne que depuis le panel overlay (`b` dans la vue Graph).

**Cause racine :** `handle_checkout()` dans `src/handler/branch.rs` (ligne ~59) vérifie
`state.view_mode == ViewMode::Graph && state.show_branch_panel`. Cette condition est
**toujours fausse** quand on est dans `ViewMode::Branches`. De plus, la fonction lit les
branches depuis les champs legacy `state.branches` / `state.branch_selected` qui ne sont
peuplés que lors de l'ouverture du panel overlay depuis la vue Graph.

## Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/handler/branch.rs` | `handle_checkout()` (ligne ~59) — condition trop restrictive |
| `src/ui/input.rs` | Mapping Enter → `BranchCheckout` (ligne ~307 pour Branches view, ligne ~194 pour overlay) |
| `src/handler/dispatcher.rs` | `BranchCheckout` → `BranchHandler` (ligne ~230) |
| `src/state/view/branches.rs` | `BranchesViewState` — `selected_branch()` méthode |
| `src/state/mod.rs` | Legacy fields `branches`, `branch_selected` |
| `src/git/branch.rs` | `checkout_branch()` (ligne ~171) |

## Plan de correction

### 1. Modifier `handle_checkout()` pour supporter `ViewMode::Branches`

Dans `src/handler/branch.rs`, `handle_checkout()` (ligne ~59) :

```rust
fn handle_checkout(state: &mut AppState) -> Result<()> {
    let branch_name = if state.view_mode == ViewMode::Branches {
        // Lire depuis le nouvel état BranchesViewState
        state.branches_view_state.selected_branch()
            .map(|b| b.name.clone())
    } else if state.view_mode == ViewMode::Graph && state.show_branch_panel {
        // Legacy: panel overlay dans la vue Graph
        state.branches.get(state.branch_selected)
            .map(|b| b.name.clone())
    } else {
        None
    };

    if let Some(name) = branch_name {
        crate::git::branch::checkout_branch(&state.repo.repo, &name)?;
        state.dirty = true;
        // Fermer le panel si applicable
        if state.show_branch_panel {
            state.show_branch_panel = false;
        }
        state.flash_message = Some(format!("Switched to branch '{}'", name));
    }
    Ok(())
}
```

### 2. Vérification

- [ ] Dans la vue Branches, sélectionner une branche et appuyer sur Enter → checkout effectué
- [ ] Un message flash confirme le changement de branche
- [ ] Le checkout depuis le panel overlay (vue Graph + `b`) fonctionne toujours
- [ ] Le checkout de la branche courante ne provoque pas d'erreur
- [ ] L'état de la vue se rafraîchit après le checkout (marqueur de branche courante mis à jour)
