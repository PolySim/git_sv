# STEP-08 : Conflict mode Block — Espace pour sélectionner, Enter pour valider le fichier

## Problème

En mode Block de la vue Conflits :
- **`Enter`** fait actuellement le toggle de sélection d'un block (ours/theirs) — il devrait être réservé pour **valider le fichier** (écrire la résolution sur disque).
- **`Espace`** ne fait rien — il devrait servir à **sélectionner/désélectionner un block**.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/ui/input.rs` | 379-388 | `Enter` → `ConflictEnterResolve` (sert actuellement de toggle) |
| `src/handler/conflict.rs` | 879-918 | `handle_enter_resolve` — en mode Block, toggle la résolution |
| `src/handler/conflict.rs` | 396-413 | `handle_mark_resolved` — écrit la résolution sur disque |
| `src/ui/input.rs` | 315-395 | Aucun binding pour `Espace` dans `map_conflicts_key` |

## Analyse

Actuellement dans `handle_enter_resolve` (`conflict.rs:892-913`), en mode Block :

```rust
ConflictResolutionMode::Block => {
    let section_idx = conflicts.section_selected;
    if let Some(conflict) = file.conflicts.get_mut(section_idx) {
        match conflicts.panel_focus {
            ConflictPanelFocus::OursPanel => {
                if conflict.resolution == Some(ConflictResolution::Ours) {
                    conflict.resolution = None; // Désélectionner
                } else {
                    conflict.resolution = Some(ConflictResolution::Ours);
                }
            }
            // ... même chose pour Theirs
        }
    }
}
```

C'est un toggle qui devrait être sur `Espace`, pas sur `Enter`.

## Solution proposée

### 1. Ajouter le binding `Espace` pour le toggle en mode Block

**Modifier `src/ui/input.rs`** — `map_conflicts_key` :

```rust
KeyCode::Char(' ') => {
    match resolution_mode {
        ConflictResolutionMode::Block => {
            if matches!(
                panel_focus,
                Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel)
            ) {
                Some(AppAction::ConflictEnterResolve)  // Toggle block (temporaire, voir point 3)
            } else {
                None
            }
        }
        ConflictResolutionMode::Line => {
            // Déjà géré dans STEP-06
            if matches!(
                panel_focus,
                Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel)
            ) {
                Some(AppAction::ConflictToggleLine)
            } else {
                None
            }
        }
        _ => None,
    }
}
```

### 2. Changer le comportement de `Enter` en mode Block

**Modifier `src/ui/input.rs`** — le binding `Enter` pour le mode Block :

En mode Block, `Enter` ne devrait plus appeler `ConflictEnterResolve`. Il devrait :
- Vérifier que toutes les sections ont une résolution choisie
- Écrire le fichier résolu sur disque (comme `ConflictResolveFile` / `MarkResolved`)

```rust
KeyCode::Enter => match panel_focus {
    Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel) => {
        match resolution_mode {
            ConflictResolutionMode::File => Some(AppAction::ConflictEnterResolve),
            ConflictResolutionMode::Block => Some(AppAction::ConflictResolveFile), // ← Valider le fichier
            ConflictResolutionMode::Line => Some(AppAction::ConflictResolveFile),  // ← Aussi pour Ligne
        }
    }
    _ => None,
},
```

### 3. Créer une nouvelle action pour le toggle de block (optionnel, plus propre)

Créer `AppAction::ConflictToggleBlock` qui fait le toggle sans la logique de résolution automatique, puis mapper `Espace` dessus en mode Block.

## Ordre d'implémentation

1. Ajouter le binding `Espace` dans `map_conflicts_key` (mode Block et mode Line)
2. Modifier le binding `Enter` en mode Block pour valider le fichier au lieu de toggler
3. Vérifier que `ConflictResolveFile` → `handle_mark_resolved` fonctionne correctement
4. Mettre à jour la help bar dans `conflicts_view.rs` pour refléter les nouveaux raccourcis
5. Tester le nouveau flux : Espace pour toggle → Enter pour valider

## Critère de validation

- En mode Block, `Espace` toggle la sélection ours/theirs pour le block courant
- En mode Block, `Enter` valide le fichier et écrit la résolution sur disque
- `Enter` affiche un message si toutes les sections ne sont pas encore résolues
- La help bar affiche les bons raccourcis
- En mode File, `Enter` continue à fonctionner comme avant (résolution directe)
