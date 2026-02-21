# STEP-03 : Corriger le scroll des panneaux Ours/Theirs/Result

## Problème

Quand un fichier en conflit est long, on ne peut pas scroller dans les panneaux Ours, Theirs et Result. Le contenu est tronqué au-delà de la hauteur visible. Les champs `ours_scroll`, `theirs_scroll` et `result_scroll` existent dans `ConflictsState` mais ne sont jamais appliqués au rendu.

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/ui/conflicts_view.rs` | `render_ours_panel()` (~166-258) | Appliquer `.scroll()` au `Paragraph` |
| `src/ui/conflicts_view.rs` | `render_theirs_panel()` (~261-357) | Appliquer `.scroll()` au `Paragraph` |
| `src/ui/conflicts_view.rs` | `render_result_panel()` (~362-438) | Appliquer `.scroll()` au `Paragraph` |
| `src/event.rs` | Handlers de navigation (~2230+) | S'assurer que le scroll suit la sélection (section ou ligne) |
| `src/state.rs` | `ConflictsState` (~442-443) | Vérifier que `ours_scroll`, `theirs_scroll`, `result_scroll` sont correctement typés |

## Détail des modifications

### 1. `src/ui/conflicts_view.rs` — Appliquer le scroll au rendu

Pour chaque panneau, ajouter `.scroll()` au `Paragraph` :

```rust
// render_ours_panel()
let paragraph = Paragraph::new(lines)
    .block(block)
    .scroll((state.ours_scroll as u16, 0)); // <-- AJOUTER

// render_theirs_panel()
let paragraph = Paragraph::new(lines)
    .block(block)
    .scroll((state.theirs_scroll as u16, 0)); // <-- AJOUTER

// render_result_panel()
let paragraph = Paragraph::new(lines)
    .block(block)
    .scroll((state.result_scroll as u16, 0)); // <-- AJOUTER
```

### 2. `src/event.rs` — Auto-scroll vers la sélection

Quand l'utilisateur navigue entre sections ou lignes, le scroll doit suivre automatiquement pour que l'élément sélectionné reste visible.

Dans les handlers `handle_conflict_next_section()`, `handle_conflict_prev_section()`, `handle_conflict_next_line()`, `handle_conflict_prev_line()` :

```rust
// Après avoir mis à jour section_selected ou line_selected,
// calculer la ligne visuelle de la sélection et ajuster le scroll :
fn auto_scroll(scroll: &mut usize, selected_line: usize, panel_height: usize) {
    // Si la sélection est au-dessus de la zone visible
    if selected_line < *scroll {
        *scroll = selected_line;
    }
    // Si la sélection est en-dessous de la zone visible
    if selected_line >= *scroll + panel_height {
        *scroll = selected_line - panel_height + 1;
    }
}
```

**Note** : La hauteur du panneau doit être calculée depuis la taille du terminal. On peut stocker `panel_height` dans `ConflictsState` ou le passer en paramètre depuis le rendu.

### 3. Scroll synchronisé Ours/Theirs

Les panneaux Ours et Theirs affichent les mêmes sections de conflit. Quand on navigue dans l'un, le scroll de l'autre devrait suivre pour garder les sections alignées.

```rust
// Quand on change de section dans n'importe quel panneau :
state.ours_scroll = calculate_scroll_for_section(section_idx, &file.conflicts, panel_height);
state.theirs_scroll = state.ours_scroll; // Synchroniser
```

### 4. Scroll du panneau Result

Le panneau Result a déjà des handlers `ConflictResultScrollDown` / `ConflictResultScrollUp` qui incrémentent/décrémentent `result_scroll`. Il faut juste appliquer la valeur au rendu (point 1) et borner la valeur max :

```rust
// Dans le handler scroll :
let max_scroll = total_lines.saturating_sub(panel_height);
state.result_scroll = state.result_scroll.min(max_scroll);
```

## Tests

- Ouvrir un fichier en conflit avec plus de lignes que la hauteur du panneau.
- Naviguer vers la dernière section : le scroll doit suivre.
- Vérifier que le scroll ne dépasse pas les bornes (pas de zone vide en bas).
- Vérifier que les panneaux Ours et Theirs restent synchronisés.
