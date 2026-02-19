# STEP-005 — Feature : Navigation dans les branches distantes

## Problème

Dans la vue Branches (vue 3), quand on active l'affichage des branches distantes avec `R` (`ToggleRemoteBranches`), on peut voir les branches remote mais on ne peut pas naviguer dedans (les sélectionner, voir leurs détails, etc.).

## Fichiers concernés

- `src/state.rs` — `BranchesViewState` (l275-309) : `show_remote`, `remote_branches`, `branch_selected`
- `src/ui/input.rs` — `map_branches_key()` (l219-274) : keybindings de la section Branches
- `src/ui/branches_view.rs` — Rendu de la liste des branches
- `src/event.rs` — Handlers des actions de navigation

## Analyse

L'état `BranchesViewState` a un seul index `branch_selected` qui est utilisé pour les branches locales. Quand `show_remote` est activé, les branches distantes sont affichées mais la navigation (j/k) ne parcourt que les branches locales. Il n'y a pas de mécanisme pour naviguer entre la liste locale et la liste remote.

## Solution

### Option A — Liste unifiée

Fusionner les branches locales et distantes dans une seule liste quand `show_remote` est activé. `branch_selected` indexe alors la liste combinée.

```rust
// Dans event.rs, lors de MoveDown/MoveUp en section Branches
let total_branches = if state.branches_view_state.show_remote {
    state.branches_view_state.local_branches.len()
        + state.branches_view_state.remote_branches.len()
} else {
    state.branches_view_state.local_branches.len()
};
```

### Option B — Index séparé pour les remotes

Ajouter un `remote_branch_selected: usize` et un flag `is_in_remote_section: bool` dans `BranchesViewState`. Quand l'utilisateur navigue au-delà de la dernière branche locale, passer automatiquement à la section remote.

### Option C — Sous-sections avec séparateur

Afficher les branches locales et distantes comme deux groupes avec un séparateur visuel. Utiliser un seul index mais avec une logique de "saut" du séparateur.

### Recommandation

Option A est la plus simple et la plus intuitive. Le rendu dans `branches_view.rs` affiche déjà les deux listes, il suffit de les combiner pour la navigation.

### Actions sur les branches distantes

Quand une branche distante est sélectionnée, les actions disponibles changent :
- `Enter` : Créer une branche locale de tracking et checkout
- `d` : Pas de suppression (ou supprimer le remote tracking ref)
- `m` : Merge depuis la branche distante

## Tests

- Naviguer dans les branches locales avec `show_remote` désactivé
- Activer `show_remote` et naviguer dans toute la liste (locales + distantes)
- Checkout d'une branche distante crée bien une branche locale de tracking
- Le séparateur visuel entre locales et distantes est correct
