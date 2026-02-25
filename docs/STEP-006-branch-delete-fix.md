# STEP-006 : Vue Branch — Delete branch ne fait rien

## Problème

Dans la vue Branches (`ViewMode::Branches`), appuyer sur `d` pour supprimer une branche
ne fait rien. Aucune boîte de confirmation n'apparaît.

**Cause racine :** `handle_delete()` dans `src/handler/branch.rs` (ligne ~91) lit la branche
depuis les champs legacy `state.branches.get(state.branch_selected)`. Ces champs ne sont
peuplés que lors de l'ouverture du panel overlay (`b` dans la vue Graph). En `ViewMode::Branches`,
`state.branches` est vide, donc le `if let Some(...)` ne match pas et rien ne se passe.

## Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/handler/branch.rs` | `handle_delete()` (ligne ~91) — utilise les champs legacy |
| `src/state/view/branches.rs` | `BranchesViewState` — `selected_branch()` pour obtenir la branche sélectionnée |
| `src/state/mod.rs` | Legacy fields `branches`, `branch_selected` |
| `src/handler/dispatcher.rs` | `handle_confirm_action()` (ligne ~649) — exécution effective du delete |
| `src/ui/confirm_dialog.rs` | `ConfirmAction::BranchDelete(name)` — boîte de confirmation |
| `src/git/branch.rs` | `delete_branch()` (ligne ~178) |

## Plan de correction

### 1. Modifier `handle_delete()` pour supporter `ViewMode::Branches`

Dans `src/handler/branch.rs`, `handle_delete()` (ligne ~91) :

```rust
fn handle_delete(state: &mut AppState) -> Result<()> {
    let branch_name = if state.view_mode == ViewMode::Branches {
        // Lire depuis le nouvel état BranchesViewState
        state.branches_view_state.selected_branch()
            .map(|b| b.name.clone())
    } else if state.view_mode == ViewMode::Graph && state.show_branch_panel {
        // Legacy: panel overlay
        state.branches.get(state.branch_selected)
            .map(|b| b.name.clone())
    } else {
        None
    };

    if let Some(name) = branch_name {
        state.pending_confirmation = Some(ConfirmAction::BranchDelete(name));
    }
    Ok(())
}
```

### 2. Empêcher la suppression de la branche courante

Ajouter une vérification avant de proposer la confirmation :

```rust
if let Some(branch) = selected_branch {
    if branch.is_head {
        state.flash_message = Some("Impossible de supprimer la branche courante".into());
        return Ok(());
    }
    state.pending_confirmation = Some(ConfirmAction::BranchDelete(branch.name.clone()));
}
```

### 3. Rafraîchir la liste après suppression

Dans `handle_confirm_action()` (dispatcher.rs, ligne ~649), après la suppression réussie,
s'assurer que `state.dirty = true` et que les listes de branches sont rechargées.
Vérifier que `branches_view_state` est aussi rafraîchi (pas seulement les champs legacy).

### 4. Vérification

- [ ] Dans la vue Branches, sélectionner une branche non-courante → `d` → boîte de confirmation apparaît
- [ ] Confirmer avec `y` → la branche est supprimée, la liste se rafraîchit
- [ ] Annuler avec `n` ou `Esc` → rien ne se passe
- [ ] Tenter de supprimer la branche courante → message d'erreur approprié
- [ ] La suppression depuis le panel overlay (vue Graph) fonctionne toujours
