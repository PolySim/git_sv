# STEP-10 : Permettre de réaccéder à la vue conflits

## Problème

Quand l'utilisateur quitte la vue conflits (`q` / `Esc`), le merge est **avorté** (`abort_merge()`) et l'état des conflits est détruit (`conflicts_state = None`). Il est impossible de revenir à la vue conflits sans refaire l'opération de merge.

L'utilisateur veut pouvoir :
1. Quitter la vue conflits **sans avorter le merge** (juste changer de vue).
2. Y revenir via la touche `4` ou l'onglet "Conflits" dans la barre de navigation.

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/ui/input.rs` | `map_conflicts_key()` (~383-530) | Changer le comportement de `q` et `Esc` |
| `src/event.rs` | `handle_conflict_abort()` (~2540) | Séparer "quitter la vue" et "avorter le merge" |
| `src/state.rs` | `AppAction` | Ajouter `ConflictLeaveView` distinct de `ConflictAbort` |
| `src/ui/nav_bar.rs` | Nav bar (~22-99) | S'assurer que l'onglet "4:Conflits" reste visible tant que `conflicts_state.is_some()` |

## Détail des modifications

### 1. Séparer "quitter la vue" et "avorter le merge"

**Nouveau comportement :**

| Touche | Action | Effet |
|--------|--------|-------|
| `q` / `Esc` | `ConflictLeaveView` | Change la vue vers Graph, **conserve** `conflicts_state` |
| `Ctrl+q` ou `A` | `ConflictAbort` | Ouvre un dialogue de confirmation, puis avorte le merge |
| `4` (depuis autre vue) | Retour à la vue Conflits | Restaure `ViewMode::Conflicts` si `conflicts_state.is_some()` |

### 2. `src/state.rs` — Nouvelle action

```rust
pub enum AppAction {
    // ...
    ConflictLeaveView,  // Quitter la vue sans avorter
    ConflictAbort,       // Avorter le merge (avec confirmation)
    // ...
}
```

### 3. `src/ui/input.rs` — Nouveau mapping

```rust
// Dans map_conflicts_key() :
KeyCode::Char('q') | KeyCode::Esc => {
    if state.is_editing {
        Some(AppAction::ConflictStopEditing) // En édition, Esc quitte l'édition
    } else {
        Some(AppAction::ConflictLeaveView) // Sinon, quitte la vue (pas le merge)
    }
}
KeyCode::Char('A') => Some(AppAction::ConflictAbort), // Avorter le merge
```

### 4. `src/event.rs` — Handler `ConflictLeaveView`

```rust
fn handle_conflict_leave_view(&mut self) {
    // NE PAS appeler abort_merge()
    // NE PAS mettre conflicts_state à None
    // Juste changer de vue
    self.state.view_mode = ViewMode::Graph;
}
```

### 5. `src/event.rs` — Handler `ConflictAbort` avec confirmation

```rust
fn handle_conflict_abort(&mut self) {
    // Ouvrir un dialogue de confirmation
    self.state.pending_confirmation = Some(ConfirmAction::AbortMerge);
}

// Dans handle_confirm_action() :
ConfirmAction::AbortMerge => {
    abort_merge(&self.repo)?;
    self.state.conflicts_state = None;
    self.state.view_mode = ViewMode::Graph;
    self.state.flash_message = Some("Merge avorté.".to_string());
}
```

### 6. `src/ui/nav_bar.rs` — Onglet persistant

L'onglet "4:Conflits" dans la barre de navigation doit :
- Être visible tant que `conflicts_state.is_some()` (déjà le cas, vérifier)
- Avoir un indicateur visuel (ex : rouge ou clignotant) si des conflits non résolus existent
- Au clic sur `4` : basculer vers `ViewMode::Conflicts`

```rust
// Dans nav_bar render :
if app.state.conflicts_state.is_some() {
    let unresolved = count_unresolved(&app.state.conflicts_state);
    let label = if unresolved > 0 {
        format!("4:Conflits ({})", unresolved) // Montrer le nombre de conflits restants
    } else {
        "4:Conflits ✓".to_string()
    };
    // Couleur rouge si non résolu, verte si tout résolu
}
```

### 7. Détection de conflits existants au démarrage

Si l'utilisateur ferme et rouvre `git_sv` alors qu'un merge est en cours :

```rust
// Au démarrage dans main.rs ou app init :
if repo.state() == RepositoryState::Merge {
    // Recharger les conflits depuis l'index
    let files = list_all_merge_files(&repo)?;
    if !files.is_empty() {
        app.state.conflicts_state = Some(ConflictsState::new(files, "Merge en cours", ...));
    }
}
```

## Tests

- Ouvrir la vue conflits, appuyer sur `q` : on revient au graph, le merge n'est pas avorté.
- Appuyer sur `4` : on revient à la vue conflits avec l'état conservé.
- Appuyer sur `A` : dialogue de confirmation, puis abort si confirmé.
- Fermer et rouvrir `git_sv` pendant un merge : les conflits sont détectés et la vue est accessible.
- Vérifier que l'onglet "4:Conflits" affiche le nombre de conflits restants.
