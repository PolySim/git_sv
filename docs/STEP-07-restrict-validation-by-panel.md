# STEP-07 : Restreindre la validation par panneau

## Problème

Actuellement, les touches `o` (ours), `t` (theirs) et `b` (both) fonctionnent depuis les panneaux Ours **et** Theirs indifféremment. L'utilisateur veut que la touche `Enter` sur le panneau Ours ne puisse valider que des modifications "ours", et inversement pour Theirs.

## Prérequis

- STEP-05 (mode Fichier)
- STEP-06 (mode Ligne)

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/ui/input.rs` | `map_conflicts_key()` (~383-530) | Supprimer `o`/`t`/`b`, remplacer par `Enter` contextuel |
| `src/event.rs` | Handlers choose_ours/theirs/both (~2237-2293) | Adapter la logique `Enter` selon le panneau et le mode |
| `src/state.rs` | `AppAction` | Éventuellement remplacer/simplifier les actions |

## Nouvelle logique

### Comportement de `Enter` selon le panneau et le mode

| Panneau | Mode Fichier | Mode Bloc | Mode Ligne |
|---------|-------------|-----------|------------|
| **FileList** | - | - | - |
| **Ours** | Résout tout le fichier en Ours | Résout la section courante en Ours | Toggle la ligne courante (ours) |
| **Theirs** | Résout tout le fichier en Theirs | Résout la section courante en Theirs | Toggle la ligne courante (theirs) |
| **Result** | - | - | - |

**Les touches `o`, `t`, `b` sont supprimées.** La sémantique est portée par le panneau sur lequel on se trouve.

### Cas spécial : "Both"

Pour garder la possibilité de choisir "les deux côtés" pour une section (ours + theirs), on peut :
- Ajouter une touche dédiée `b` qui reste disponible **uniquement en mode Bloc** et fonctionne depuis les deux panneaux
- Ou : en mode Ligne, l'utilisateur sélectionne simplement les lignes des deux côtés qu'il veut garder (plus naturel)

## Détail des modifications

### 1. `src/ui/input.rs` — Nouveau mapping

```rust
// Panneau Ours/Theirs :
KeyCode::Enter => {
    match state.resolution_mode {
        ConflictResolutionMode::File => Some(AppAction::ConflictEnterResolve),
        ConflictResolutionMode::Block => Some(AppAction::ConflictEnterResolve),
        ConflictResolutionMode::Line => Some(AppAction::ConflictEnterResolve),
    }
}

// Garder 'b' pour "Both" en mode Bloc uniquement :
KeyCode::Char('b') if mode == Block => Some(AppAction::ConflictChooseBoth),
```

### 2. `src/event.rs` — Handler unifié `ConflictEnterResolve`

```rust
fn handle_conflict_enter_resolve(&mut self) {
    let cs = self.state.conflicts_state.as_mut().unwrap();
    
    let side = match cs.focus {
        ConflictPanelFocus::OursPanel => ResolutionSide::Ours,
        ConflictPanelFocus::TheirsPanel => ResolutionSide::Theirs,
        _ => return, // Pas de résolution depuis FileList ou Result
    };
    
    match cs.resolution_mode {
        ConflictResolutionMode::File => {
            // Résoudre toutes les sections du fichier
            self.resolve_all_sections(side);
        }
        ConflictResolutionMode::Block => {
            // Résoudre la section courante
            self.resolve_current_section(side);
        }
        ConflictResolutionMode::Line => {
            // Toggle la ligne courante
            self.toggle_current_line(side);
        }
    }
}
```

### 3. `src/ui/conflicts_view.rs` — Mise à jour de la barre d'aide

Mettre à jour l'aide en bas pour refléter les nouvelles touches :

```
Enter:Valider  Tab:Panneau  ↑↓:Nav  F/B/L:Mode  b:Les deux (Bloc)  i:Éditer  V:Finaliser  q:Quitter
```

## Tests

- Sur le panneau Ours en mode Bloc : `Enter` doit résoudre la section en Ours.
- Sur le panneau Theirs en mode Bloc : `Enter` doit résoudre la section en Theirs.
- Sur le panneau Ours en mode Fichier : `Enter` doit résoudre tout le fichier en Ours.
- Vérifier que `Enter` ne fait rien depuis FileList ou Result.
- Vérifier que `o` et `t` ne sont plus des raccourcis valides.
