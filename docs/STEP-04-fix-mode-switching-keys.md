# STEP-04 : Corriger les touches de changement de mode (F/B/L)

## Problème

Actuellement les touches `F`, `B` et `L` font toutes la même chose : elles **cyclent** entre les modes (Block -> Line -> File -> Block). L'utilisateur s'attend à ce que chaque touche active directement le mode correspondant. De plus, il faut un indicateur visuel du mode actif.

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/state.rs` | `AppAction` enum (~137-177) | Remplacer `ConflictSwitchMode` par 3 actions distinctes |
| `src/ui/input.rs` | `map_conflicts_key()` (~498-500) | Mapper `F` -> File, `B` -> Block, `L` -> Line |
| `src/event.rs` | `handle_conflict_switch_mode()` (~2564) | Remplacer par 3 handlers ou un handler avec paramètre |
| `src/ui/conflicts_view.rs` | Barre d'aide ou status bar | Afficher le mode actif |

## Détail des modifications

### 1. `src/state.rs` — Nouvelles actions

```rust
// Remplacer :
ConflictSwitchMode,

// Par :
ConflictSetModeFile,
ConflictSetModeBlock,
ConflictSetModeLine,
```

### 2. `src/ui/input.rs` — Mapping direct

```rust
// AVANT
KeyCode::Char('F') => Some(AppAction::ConflictSwitchMode),
KeyCode::Char('B') => Some(AppAction::ConflictSwitchMode),
KeyCode::Char('L') => Some(AppAction::ConflictSwitchMode),

// APRÈS
KeyCode::Char('F') => Some(AppAction::ConflictSetModeFile),
KeyCode::Char('B') => Some(AppAction::ConflictSetModeBlock),
KeyCode::Char('L') => Some(AppAction::ConflictSetModeLine),
```

### 3. `src/event.rs` — Handlers directs

```rust
AppAction::ConflictSetModeFile => {
    if let Some(ref mut cs) = self.state.conflicts_state {
        cs.resolution_mode = ConflictResolutionMode::File;
        cs.line_selected = 0;
    }
}
AppAction::ConflictSetModeBlock => {
    if let Some(ref mut cs) = self.state.conflicts_state {
        cs.resolution_mode = ConflictResolutionMode::Block;
        cs.line_selected = 0;
    }
}
AppAction::ConflictSetModeLine => {
    if let Some(ref mut cs) = self.state.conflicts_state {
        cs.resolution_mode = ConflictResolutionMode::Line;
        cs.line_selected = 0;
    }
}
```

### 4. `src/ui/conflicts_view.rs` — Indicateur visuel du mode actif

Ajouter dans la barre de status (en haut) ou dans la barre d'aide (en bas) l'indication du mode actuel :

```rust
// Dans la barre de status ou en bas :
let mode_text = match state.resolution_mode {
    ConflictResolutionMode::File => "[Mode: Fichier]",
    ConflictResolutionMode::Block => "[Mode: Bloc]",
    ConflictResolutionMode::Line => "[Mode: Ligne]",
};
```

Afficher ce texte dans la barre d'aide à côté des raccourcis :

```
F:Fichier  B:Bloc  L:Ligne  |  Mode actif: Bloc
```

## Tests

- Appuyer sur `F` : vérifier que le mode passe directement à "Fichier".
- Appuyer sur `B` : vérifier que le mode passe directement à "Bloc".
- Appuyer sur `L` : vérifier que le mode passe directement à "Ligne".
- Vérifier que l'indicateur visuel se met à jour immédiatement.
- Vérifier que `line_selected` est remis à 0 à chaque changement de mode.
