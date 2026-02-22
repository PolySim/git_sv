# STEP-1 : La vue Branches n'affiche aucune branche

## Problème

Quand on presse `3` pour basculer en `ViewMode::Branches`, la liste de branches est **vide**. Aucune branche locale ni distante n'apparaît.

## Cause racine

Il y a **deux systèmes de stockage de branches** dans `AppState`, et la migration vers le nouveau système est incomplète :

| Ancien système (legacy) | Nouveau système |
|---|---|
| `state.branches: Vec<BranchInfo>` | `state.branches_view_state.local_branches: ListSelection<BranchInfo>` |
| `state.branch_selected: usize` | `state.branches_view_state.remote_branches: ListSelection<BranchInfo>` |
| `state.show_branch_panel: bool` | (toujours rendu en `ViewMode::Branches`) |

La vue `branches_view.rs` lit depuis `state.branches_view_state.local_branches` et `state.branches_view_state.remote_branches`, mais **aucun code ne peuple jamais ces champs**.

### Détail du flux cassé

1. **`SwitchToBranches`** dans `dispatcher.rs:~233` fait :
   ```rust
   ctx.state.view_mode = ViewMode::Branches;
   ctx.state.dirty = true;
   ```

2. **`dirty = true`** déclenche `EventHandler::refresh()` dans `handler/mod.rs:~87`, mais cette fonction ne rafraîchit que : `current_branch`, `graph`, `status_entries`, `commit_files`, `staging`. **Aucun chargement de branches.**

3. **`render_branches_list()`** dans `ui/branches_view.rs:~137` itère sur `state.local_branches` et `state.remote_branches` → les deux sont vides → la liste est vide.

4. Le seul endroit qui charge des branches est **`BranchHandler::handle_list()`** dans `handler/branch.rs:~43`, mais il :
   - Ne se déclenche que sur `BranchAction::List` (toggle du panneau overlay, pas de la vue Branches)
   - Écrit dans `state.branches` (champ **legacy**), pas dans `state.branches_view_state.local_branches`

5. **`handle_branch_list()`** dans `handler/git.rs:~217` est un **stub vide** (`Ok(())`).

## Fichiers concernés

| Fichier | Ligne(s) | Problème |
|---|---|---|
| `src/handler/mod.rs` | `refresh()` ~87-125 | Ne charge pas les branches dans `branches_view_state` |
| `src/handler/git.rs` | `handle_branch_list()` ~217-220 | Stub vide (no-op) |
| `src/handler/branch.rs` | `handle_list()` ~43-57 | Peuple `state.branches` (legacy) au lieu de `branches_view_state` |
| `src/ui/branches_view.rs` | `render_branches_list()` ~137 | Lit depuis `branches_view_state` (jamais peuplé) |
| `src/state/view/branches.rs` | `BranchesViewState` | Déclare les bons champs mais ils ne sont jamais initialisés |

## Correction proposée

### 1. Ajouter le chargement des branches dans `EventHandler::refresh()` (`handler/mod.rs`)

Quand `view_mode == ViewMode::Branches`, charger les branches, worktrees et stashes :

```rust
fn refresh(&mut self) -> Result<()> {
    // ... code existant ...

    // Charger les données de la vue branches
    if self.state.view_mode == ViewMode::Branches {
        match crate::git::branch::list_all_branches(&self.state.repo.repo) {
            Ok((local, remote)) => {
                self.state.branches_view_state.local_branches.set_items(local);
                self.state.branches_view_state.remote_branches.set_items(remote);
            }
            Err(e) => {
                self.state.set_flash_message(format!("Erreur chargement branches: {}", e));
            }
        }

        // Charger les worktrees
        match crate::git::worktree::list_worktrees(&self.state.repo.repo) {
            Ok(worktrees) => {
                self.state.branches_view_state.worktrees.set_items(worktrees);
            }
            Err(_) => {}
        }

        // Charger les stashes
        match crate::git::stash::list_stashes(&mut self.state.repo.repo) {
            Ok(stashes) => {
                self.state.branches_view_state.stashes.set_items(stashes);
            }
            Err(_) => {}
        }
    }

    self.state.dirty = false;
    Ok(())
}
```

> **Note :** Vérifier que `ListSelection<T>` a bien une méthode `set_items()` ou équivalent. Si elle n'existe pas, la créer dans `src/state/selection.rs`. Si le constructeur est `ListSelection::from(items)`, remplacer le champ entier :
> ```rust
> self.state.branches_view_state.local_branches = ListSelection::new(local);
> ```

### 2. Mettre à jour `handle_list()` dans `handler/branch.rs`

Le `handle_list()` actuel peuple seulement le champ legacy. Il faut aussi peupler `branches_view_state` :

```rust
fn handle_list(state: &mut AppState) -> Result<()> {
    if matches!(state.view_mode, ViewMode::Graph | ViewMode::Branches) {
        state.show_branch_panel = !state.show_branch_panel;
        if state.show_branch_panel {
            match crate::git::branch::list_all_branches(&state.repo.repo) {
                Ok((local, remote)) => {
                    // Legacy (pour le panneau overlay en Graph view)
                    state.branches = local.clone();
                    state.branch_selected = 0;
                    // Nouveau système (pour la vue Branches)
                    state.branches_view_state.local_branches.set_items(local);
                    state.branches_view_state.remote_branches.set_items(remote);
                }
                Err(e) => {
                    state.set_flash_message(format!("Erreur: {}", e));
                }
            }
        }
    }
    Ok(())
}
```

## Vérification

Après correction :
1. `cargo build` compile sans erreur
2. `cargo test` passe
3. Lancer l'app, presser `3` → la liste de branches locales s'affiche
4. Presser `R` pour toggle remote → les branches distantes apparaissent aussi
5. La navigation `j/k` fonctionne dans la liste
