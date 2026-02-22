# STEP-07 : Conflict mode Ligne — Naviguer entre les lignes d'autres blocks

## Problème

En mode Ligne de la vue Conflits, la navigation avec `j`/`k` (haut/bas) est limitée aux lignes du block courant (`section_selected`). Une fois arrivé à la dernière ligne d'un block, on ne peut pas passer au block suivant. L'utilisateur doit manuellement changer de mode ou de section, ce qui n'est pas intuitif.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/ui/input.rs` | 342-346 | Mode Ligne : `Down` → `ConflictLineDown`, `Up` → `ConflictLineUp` |
| `src/handler/conflict.rs` | 547-570 | `handle_line_down` — ne dépasse pas les bornes du block courant |
| `src/handler/conflict.rs` | 572-579 | `handle_line_up` — ne passe pas au block précédent |

## Analyse

`handle_line_down` (`conflict.rs:547-570`) :

```rust
fn handle_line_down(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let max_lines = /* nombre de lignes du block courant */;
        if conflicts.line_selected < max_lines.saturating_sub(1) {
            conflicts.line_selected += 1;
        }
        // ← Ici, rien ne se passe si on est à la dernière ligne
    }
    Ok(())
}
```

La navigation est bloquée à `max_lines - 1`. Il n'y a pas de logique pour passer au block suivant.

`handle_line_up` (`conflict.rs:572-579`) :

```rust
fn handle_line_up(state: &mut AppState) -> Result<()> {
    if let Some(ref mut conflicts) = state.conflicts_state {
        if conflicts.line_selected > 0 {
            conflicts.line_selected -= 1;
        }
        // ← Rien si on est à la première ligne (pas de passage au block précédent)
    }
    Ok(())
}
```

## Solution proposée

1. **Modifier `src/handler/conflict.rs`** — `handle_line_down` :

   ```rust
   fn handle_line_down(state: &mut AppState) -> Result<()> {
       if let Some(conflicts) = &mut state.conflicts_state {
           let max_lines = /* lignes du block courant */;

           if conflicts.line_selected < max_lines.saturating_sub(1) {
               conflicts.line_selected += 1;
           } else {
               // Passer au block suivant si disponible
               let file = &conflicts.all_files[conflicts.file_selected];
               if conflicts.section_selected + 1 < file.conflicts.len() {
                   conflicts.section_selected += 1;
                   conflicts.line_selected = 0;
               }
           }
       }
       Ok(())
   }
   ```

2. **Modifier `src/handler/conflict.rs`** — `handle_line_up` :

   ```rust
   fn handle_line_up(state: &mut AppState) -> Result<()> {
       if let Some(conflicts) = &mut state.conflicts_state {
           if conflicts.line_selected > 0 {
               conflicts.line_selected -= 1;
           } else if conflicts.section_selected > 0 {
               // Passer au block précédent
               conflicts.section_selected -= 1;
               let file = &conflicts.all_files[conflicts.file_selected];
               let prev_section = &file.conflicts[conflicts.section_selected];
               let max_lines = match conflicts.panel_focus {
                   ConflictPanelFocus::OursPanel => prev_section.ours.len(),
                   ConflictPanelFocus::TheirsPanel => prev_section.theirs.len(),
                   _ => 0,
               };
               conflicts.line_selected = max_lines.saturating_sub(1);
           }
       }
       Ok(())
   }
   ```

3. **Optionnel** : ajouter un indicateur visuel dans le titre de section montrant quelle section est active et la position globale (ex: `Ligne 5/12 — Section 2/3`).

## Ordre d'implémentation

1. Modifier `handle_line_down` pour passer au block suivant en fin de block
2. Modifier `handle_line_up` pour passer au block précédent en début de block
3. S'assurer que le scroll du panneau suit la section active
4. Tester : naviguer de la dernière ligne du block 1 vers la première du block 2

## Critère de validation

- `j`/`Down` en fin de block passe à la première ligne du block suivant
- `k`/`Up` en début de block passe à la dernière ligne du block précédent
- La section sélectionnée (`section_selected`) est mise à jour automatiquement
- Le rendu visuel suit la navigation (highlight de la bonne section)
- Pas de crash si on est au dernier block ou au premier block
