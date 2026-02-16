# STEP 04 — Feature : Améliorer l'UX du Merge

## Contexte

Actuellement, lorsqu'on appuie sur `m` pour merger, un overlay s'ouvre et demande de **taper le nom** de la branche à merger. Ce n'est pas ergonomique — l'utilisateur doit connaître le nom exact.

## Comportement souhaité

### Cas 1 — Vue Branches (touche `m`)

Quand on est dans la vue Branches (vue 3) avec le focus sur une branche :
- **Merge direct** : La branche actuellement sélectionnée/focusée dans la liste est mergée dans la branche courante (HEAD).
- Pas d'overlay d'input.
- Un dialogue de confirmation s'affiche : `"Merger 'feature/x' dans 'main' ? (y/n)"`

### Cas 2 — Autres vues (Graph, Staging — touche `m`)

Quand on est dans une autre vue :
- Au lieu de l'overlay texte actuel, afficher une **liste de sélection** avec toutes les branches disponibles.
- Navigation avec `j/k` ou flèches.
- `Enter` pour confirmer le merge, `Esc` pour annuler.
- La branche courante est marquée et non sélectionnable (on ne peut pas merger une branche dans elle-même).

## Plan d'implémentation

### Étape 1 — Nouveau mode de sélection : `MergePicker`

Ajouter un nouveau focus/mode dans l'état :

**`src/state.rs`** :
```rust
/// État du sélecteur de branche pour le merge.
pub struct MergePickerState {
    /// Liste des branches disponibles (hors branche courante).
    pub branches: Vec<String>,
    /// Index de la branche sélectionnée.
    pub selected: usize,
    /// Actif ou non.
    pub is_active: bool,
}
```

Ajouter `merge_picker: Option<MergePickerState>` dans `AppState`.

### Étape 2 — Modifier `handle_merge_prompt()` dans `src/event.rs`

Selon la vue actuelle :

```rust
fn handle_merge_prompt(&mut self) -> Result<()> {
    if self.state.view_mode == ViewMode::Branches {
        // Cas 1 : Vue Branches → merge direct de la branche focusée
        let branch = self.state.branches_view_state.local_branches
            .get(self.state.branches_view_state.branch_selected);
        if let Some(branch) = branch {
            if branch.is_head {
                self.state.set_flash_message("Impossible de merger la branche courante dans elle-même".into());
            } else {
                // Demander confirmation
                self.state.pending_confirmation = Some(ConfirmAction::MergeBranch(branch.name.clone()));
            }
        }
    } else {
        // Cas 2 : Autres vues → ouvrir le sélecteur de branches
        self.open_merge_picker()?;
    }
    Ok(())
}
```

### Étape 3 — Ajouter `ConfirmAction::MergeBranch`

**`src/ui/confirm_dialog.rs`** — Ajouter la variante :
```rust
pub enum ConfirmAction {
    // ...existants...
    MergeBranch(String),
}
```

**`src/event.rs`** — Gérer la confirmation dans `handle_confirm_action()` :
```rust
ConfirmAction::MergeBranch(name) => {
    self.execute_merge(&name)?;
}
```

### Étape 4 — Implémenter le sélecteur de branches (Merge Picker)

**`src/ui/merge_picker.rs`** (nouveau fichier) — Widget de sélection :
- Overlay centré affichant la liste des branches locales
- La branche courante est grisée / non sélectionnable
- Titre : `" Merger dans '<branche_courante>' "`
- Navigation `j/k`, sélection `Enter`, annulation `Esc`

**`src/ui/input.rs`** — Ajouter les keybindings quand le merge picker est actif :
```rust
// Vérifier si le merge picker est actif
if state.merge_picker.as_ref().map_or(false, |p| p.is_active) {
    return match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MergePickerDown),
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MergePickerUp),
        KeyCode::Enter => Some(AppAction::MergePickerConfirm),
        KeyCode::Esc => Some(AppAction::MergePickerCancel),
        _ => None,
    };
}
```

**`src/state.rs`** — Ajouter les nouvelles actions :
```rust
pub enum AppAction {
    // ...existants...
    MergePickerUp,
    MergePickerDown,
    MergePickerConfirm,
    MergePickerCancel,
}
```

### Étape 5 — Ajouter le raccourci `m` dans la vue Branches

**`src/ui/input.rs`** — Dans `map_branches_key()`, section `BranchesSection::Branches` :
```rust
KeyCode::Char('m') => Some(AppAction::MergePrompt),
```

### Étape 6 — Supprimer l'ancien mode `InputAction::MergeBranch`

- Retirer `InputAction::MergeBranch` de `src/state.rs`
- Retirer le cas correspondant dans `handle_confirm_input()` de `src/event.rs`
- Retirer le titre de l'overlay dans `render_input_overlay()` de `src/ui/branches_view.rs`

### Étape 7 — Mettre à jour les barres d'aide

- Vue Branches : `"m:merge  Enter:checkout  n:new  d:delete  r:rename"`
- Vue Graph : `"m:merge"` (ouvre le picker)
- Overlay Merge Picker : `"j/k:naviguer  Enter:merger  Esc:annuler"`

## Fichiers à modifier / créer

| Fichier | Modification |
|---------|-------------|
| `src/state.rs` | Ajouter `MergePickerState`, nouvelles actions, supprimer `InputAction::MergeBranch` |
| `src/event.rs` | Modifier `handle_merge_prompt()`, ajouter handlers du picker, ajouter `ConfirmAction::MergeBranch` |
| `src/ui/input.rs` | Ajouter keybindings du merge picker, ajouter `m` dans `map_branches_key()` |
| `src/ui/merge_picker.rs` | **Nouveau** — Widget de sélection de branche pour le merge |
| `src/ui/mod.rs` | Ajouter `pub mod merge_picker;` et rendre dans l'overlay si actif |
| `src/ui/confirm_dialog.rs` | Ajouter `ConfirmAction::MergeBranch(String)` |
| `src/ui/branches_view.rs` | Supprimer le titre MergeBranch de `render_input_overlay()`, mettre à jour l'aide |

## Dépendances

- **STEP_05** (Résolution de conflits) : Si le merge génère des conflits, il faudra rediriger vers la vue de résolution au lieu de simplement afficher une erreur.

## Priorité

**Moyenne** — Amélioration d'ergonomie significative.
