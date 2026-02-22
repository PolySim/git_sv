# STEP-04 : Corriger le mode Block et Ligne (Enter + flèches)

## Problème

En mode Block ou Ligne, dans les panneaux Ours/Theirs :
1. **Enter ne fait rien** — `handle_enter_resolve` est un stub vide (`Ok(())`)
2. **Flèche bas en mode Block ne fait rien** — `handle_next_section` est un stub vide
3. **Les actions de résolution par block sont des stubs** — `handle_accept_ours_block`, `handle_accept_theirs_block`, `handle_accept_both`
4. **Toggle ligne est un stub** — `handle_toggle_line`
5. **`line_selected` n'a pas de borne supérieure** — `handle_line_down` incrémente sans limite

## Fichiers à modifier

- `src/handler/conflict.rs` — Implémenter tous les stubs

## Corrections

### 1. Implémenter `handle_next_section` (~ligne 87)

```rust
fn handle_next_section(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let file = &conflicts.all_files[conflicts.file_selected];
        let max_section = file.sections.len().saturating_sub(1);
        if conflicts.section_selected < max_section {
            conflicts.section_selected += 1;
            conflicts.line_selected = 0; // Reset la sélection de ligne
        }
    }
    Ok(())
}
```

### 2. Implémenter `handle_enter_resolve` (~ligne 334)

Comportement attendu selon le mode :
- **Mode File** : Résoudre le fichier entier selon le panneau actif (Ours → accepter ours, Theirs → accepter theirs)
- **Mode Block** : Résoudre la section courante selon le panneau actif
- **Mode Line** : Toggle l'inclusion de la ligne courante (cf. STEP-05 pour le détail du toggle)

```rust
fn handle_enter_resolve(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        match conflicts.resolution_mode {
            ConflictResolutionMode::File => {
                match conflicts.panel_focus {
                    ConflictPanelFocus::OursPanel => handle_accept_ours_file(state)?,
                    ConflictPanelFocus::TheirsPanel => handle_accept_theirs_file(state)?,
                    _ => {}
                }
            }
            ConflictResolutionMode::Block => {
                match conflicts.panel_focus {
                    ConflictPanelFocus::OursPanel => handle_accept_ours_block(state)?,
                    ConflictPanelFocus::TheirsPanel => handle_accept_theirs_block(state)?,
                    _ => {}
                }
            }
            ConflictResolutionMode::Line => {
                handle_toggle_line(state)?;
            }
        }
    }
    Ok(())
}
```

### 3. Implémenter `handle_accept_ours_block` / `handle_accept_theirs_block` (~lignes 136-141)

```rust
fn handle_accept_ours_block(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
            if let Some(section) = file.sections.get_mut(section_idx) {
                section.resolution = Some(ConflictResolution::Ours);
            }
        }
        // Mettre à jour le résultat affiché
    }
    Ok(())
}
```

Même pattern pour `handle_accept_theirs_block` avec `ConflictResolution::Theirs`.

### 4. Implémenter `handle_accept_both` (~ligne 144)

```rust
fn handle_accept_both(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
            if let Some(section) = file.sections.get_mut(section_idx) {
                section.resolution = Some(ConflictResolution::Both);
            }
        }
    }
    Ok(())
}
```

### 5. Implémenter `handle_toggle_line` (~ligne 243)

Toggle l'inclusion de la ligne courante dans `LineLevelResolution` :
```rust
fn handle_toggle_line(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let section_idx = conflicts.section_selected;
        let line_idx = conflicts.line_selected;
        match conflicts.panel_focus {
            ConflictPanelFocus::OursPanel => {
                // Toggle ours_lines_included[line_idx]
                if let Some(resolution) = &mut conflicts.line_resolutions.get_mut(section_idx) {
                    if let Some(included) = resolution.ours_lines_included.get_mut(line_idx) {
                        *included = !*included;
                    }
                }
            }
            ConflictPanelFocus::TheirsPanel => {
                // Toggle theirs_lines_included[line_idx]
                // Même pattern
            }
            _ => {}
        }
    }
    Ok(())
}
```

### 6. Borner `handle_line_down` (~ligne 248)

Ajouter une vérification de borne supérieure :
```rust
fn handle_line_down(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let max_lines = /* nombre de lignes de la section courante selon le panneau actif */;
        if conflicts.line_selected < max_lines.saturating_sub(1) {
            conflicts.line_selected += 1;
        }
    }
    Ok(())
}
```

## Vérification

```bash
cargo build
# Tester en mode Block : naviguer entre sections avec flèches, résoudre avec Enter
# Tester en mode Ligne : naviguer entre lignes, toggle avec Enter
```
