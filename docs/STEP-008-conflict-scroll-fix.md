# STEP-008 : Résolution de conflit — Scroll impossible dans les panneaux Ours/Theirs

## Problème

Dans la vue de résolution de conflit, il est impossible de scroller dans les panneaux
Ours et Theirs. Quand un fichier de conflit a plus de lignes que la hauteur du viewport,
les lignes hors écran sont inaccessibles.

**Cause racine :** Les champs `ours_scroll` et `theirs_scroll` dans `ConflictsState`
sont initialisés à 0 et ne sont **jamais modifiés par aucune action utilisateur**.
Seule une remise à 0 est faite au changement de fichier (lignes ~64-65, ~79-80 de
handler/conflict.rs).

La navigation j/k en mode Ligne (`handle_line_down`/`handle_line_up`) déplace le curseur
de sélection (`line_selected` / `section_selected`) mais **n'ajuste pas le scroll offset**.
Le widget `Paragraph::scroll((state.ours_scroll, 0))` reste toujours à 0.

### Problèmes supplémentaires

1. **`result_scroll` n'a pas de borne max** : `handle_result_scroll_down` incrémente
   sans limite, permettant de scroller au-delà du contenu.

2. **Pas de scroll souris** : `map_mouse()` dans input.rs ne gère pas `ViewMode::Conflicts`.

## Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/state/view/conflicts.rs` | `ConflictsState` — champs `ours_scroll`, `theirs_scroll`, `result_scroll` (lignes ~34-38) |
| `src/handler/conflict.rs` | `handle_line_down/up` (lignes ~726-774) — déplace `line_selected` sans ajuster le scroll |
| `src/handler/conflict.rs` | `handle_result_scroll_down/up` (lignes ~781-788) — sans borne max |
| `src/ui/conflicts_view.rs` | `render_ours_panel()` (ligne ~404) `.scroll((state.ours_scroll, 0))`, `render_theirs_panel()` (ligne ~601) |
| `src/ui/input.rs` | `map_mouse()` (lignes ~560-615) — pas de cas pour `ViewMode::Conflicts` |

## Plan de correction

### 1. Auto-scroll pour suivre `line_selected` en mode Ligne

Dans `handle_line_down()` et `handle_line_up()` (src/handler/conflict.rs, lignes ~726-774),
après avoir mis à jour `line_selected`, calculer la position visuelle de la ligne et
ajuster le scroll :

```rust
fn adjust_scroll_to_selection(state: &mut ConflictsState, visible_height: usize) {
    let line_pos = calculate_visual_line_position(state); // position absolue dans le rendu
    
    // Scroll vers le bas si la sélection dépasse le viewport
    if line_pos >= state.ours_scroll + visible_height {
        state.ours_scroll = line_pos - visible_height + 1;
    }
    // Scroll vers le haut si la sélection est au-dessus du viewport
    if line_pos < state.ours_scroll {
        state.ours_scroll = line_pos;
    }
    
    // Synchroniser theirs_scroll avec ours_scroll pour une navigation cohérente
    state.theirs_scroll = state.ours_scroll;
}
```

### 2. Stocker `visible_height` dans l'état

Ajouter un champ `visible_height: usize` à `ConflictsState` et le mettre à jour à chaque
rendu dans `render_ours_panel()` / `render_theirs_panel()` (en utilisant `area.height`).

Alternativement, calculer `visible_height` dans le handler à partir de la taille du terminal.

### 3. Borner `result_scroll`

Dans `handle_result_scroll_down()` (ligne ~781) :

```rust
fn handle_result_scroll_down(state: &mut AppState) -> Result<()> {
    let conflicts = &mut state.conflicts_state;
    let max_scroll = total_result_lines.saturating_sub(visible_height);
    if conflicts.result_scroll < max_scroll {
        conflicts.result_scroll += 1;
    }
    Ok(())
}
```

### 4. Ajouter le scroll souris pour la vue Conflicts

Dans `src/ui/input.rs`, `map_mouse()` (lignes ~560-615), ajouter un cas pour
`ViewMode::Conflicts` :

```rust
ViewMode::Conflicts => {
    match event.kind {
        MouseEventKind::ScrollDown => Some(AppAction::ConflictResultScrollDown),
        MouseEventKind::ScrollUp => Some(AppAction::ConflictResultScrollUp),
        _ => None,
    }
}
```

Idéalement, détecter dans quel panneau (ours/theirs/result) la souris se trouve et
scroller le bon panneau.

### 5. Calculer `calculate_visual_line_position`

Cette fonction doit correspondre à la logique de rendu dans `render_ours_panel()`.
Chaque section a un header ("Section N") + ses lignes. La position visuelle est :

```rust
fn calculate_visual_line_position(state: &ConflictsState) -> usize {
    let file = &state.all_files[state.file_selected];
    let mut pos = 0;
    for (section_idx, conflict) in file.conflicts.iter().enumerate() {
        pos += 1; // Header "Section N"
        if section_idx == state.section_selected {
            pos += state.line_selected;
            return pos;
        }
        pos += conflict.ours_lines.len(); // ou theirs_lines selon le panneau
    }
    pos
}
```

### 6. Vérification

- [ ] En mode Ligne, naviguer avec j/k au-delà du viewport → le panneau scrolle pour suivre la sélection
- [ ] Scroller vers le haut au-delà du viewport → le panneau suit aussi
- [ ] Le panneau Result ne peut pas scroller au-delà de son contenu
- [ ] La molette souris scrolle dans la vue Conflicts
- [ ] Le scroll se réinitialise au changement de fichier
- [ ] Pas de régression sur la navigation en mode Block et File
