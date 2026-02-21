# STEP 5 - Vue Résultat : background coloré par provenance + mode édition

## Problème

1. **Le panneau Résultat n'indique pas d'où viennent les lignes.** Toutes les lignes résolues sont affichées en blanc sans distinction. L'utilisateur ne peut pas voir quelles lignes viennent de "ours", de "theirs", ou des deux.
2. **On ne peut pas éditer directement le résultat.** Pour des petites modifications (typo, ajustement), il faut quitter l'outil et éditer manuellement.

## Fichiers à modifier

| Fichier | Rôle |
|---------|------|
| `src/git/conflict.rs` | `generate_resolved_content()` : retourner la provenance de chaque ligne |
| `src/state.rs` | `ConflictsState` : ajouter l'état du mode édition + buffer éditable |
| `src/ui/input.rs` | Ajouter les keybindings pour le mode édition dans le panneau Result |
| `src/ui/conflicts_view.rs` | Colorier les lignes selon leur provenance + mode édition |
| `src/event.rs` | Handlers pour le mode édition |

## Modifications détaillées

### 1. `src/git/conflict.rs` — Provenance des lignes

Modifier `generate_resolved_content()` pour retourner la provenance de chaque ligne :

```rust
/// Source d'une ligne dans le résultat résolu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineSource {
    /// Ligne de contexte (inchangée).
    Context,
    /// Ligne provenant de "ours".
    Ours,
    /// Ligne provenant de "theirs".
    Theirs,
    /// Marqueur de conflit non résolu.
    ConflictMarker,
}

/// Ligne résolue avec sa provenance.
#[derive(Debug, Clone)]
pub struct ResolvedLine {
    pub content: String,
    pub source: LineSource,
}

/// Génère le contenu résolu avec provenance.
pub fn generate_resolved_content_with_source(
    file: &MergeFile,
    mode: ConflictResolutionMode,
) -> Vec<ResolvedLine> {
    let mut result: Vec<ResolvedLine> = Vec::new();

    for section in &file.conflicts {
        // Contexte avant → LineSource::Context
        for line in &section.context_before {
            result.push(ResolvedLine {
                content: line.clone(),
                source: LineSource::Context,
            });
        }

        match mode {
            ConflictResolutionMode::File | ConflictResolutionMode::Block => {
                if let Some(resolution) = &section.resolution {
                    match resolution {
                        ConflictResolution::Ours => {
                            for line in &section.ours {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Ours,
                                });
                            }
                        }
                        ConflictResolution::Theirs => {
                            for line in &section.theirs {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Theirs,
                                });
                            }
                        }
                        ConflictResolution::Both => {
                            for line in &section.ours {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Ours,
                                });
                            }
                            for line in &section.theirs {
                                result.push(ResolvedLine {
                                    content: line.clone(),
                                    source: LineSource::Theirs,
                                });
                            }
                        }
                    }
                } else {
                    // Non résolu → marqueurs de conflit
                    result.push(ResolvedLine {
                        content: "<<<<<<< HEAD".into(),
                        source: LineSource::ConflictMarker,
                    });
                    // ... ours, =======, theirs, >>>>>>>
                }
            }
            ConflictResolutionMode::Line => {
                // Idem avec source par ligne
            }
        }

        // Contexte après → LineSource::Context
        for line in &section.context_after {
            result.push(ResolvedLine {
                content: line.clone(),
                source: LineSource::Context,
            });
        }
    }

    result
}
```

L'ancienne fonction `generate_resolved_content()` peut être conservée comme wrapper pour la compatibilité.

### 2. `src/ui/conflicts_view.rs` — Coloration par provenance

Modifier `build_result_content()` pour utiliser la nouvelle fonction et appliquer un **background** selon la provenance :

```rust
fn build_result_content<'a>(file: &'a MergeFile, state: &'a ConflictsState) -> Vec<Line<'a>> {
    use crate::git::conflict::{generate_resolved_content_with_source, LineSource};

    let resolved = generate_resolved_content_with_source(file, state.resolution_mode);

    resolved.iter().enumerate().map(|(idx, rline)| {
        let style = match rline.source {
            LineSource::Context => Style::default(),
            LineSource::Ours => Style::default()
                .bg(Color::Rgb(0, 40, 0)),    // Fond vert très foncé
            LineSource::Theirs => Style::default()
                .bg(Color::Rgb(0, 0, 40)),    // Fond bleu très foncé
            LineSource::ConflictMarker => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        };

        // En mode édition, mettre en évidence la ligne du curseur
        let style = if state.is_editing && state.edit_cursor_line == idx {
            style.add_modifier(Modifier::UNDERLINED)
        } else {
            style
        };

        Line::from(Span::styled(&rline.content, style))
    }).collect()
}
```

> Note : les couleurs RGB (`Color::Rgb`) ne fonctionnent que dans les terminaux supportant les 24-bit colors. Pour une compatibilité maximale, on peut aussi utiliser `Color::Indexed(22)` (vert foncé) et `Color::Indexed(17)` (bleu foncé).

### 3. `src/state.rs` — État du mode édition

Ajouter dans `ConflictsState` :

```rust
pub struct ConflictsState {
    // ... champs existants ...

    /// Mode édition actif dans le panneau résultat.
    pub is_editing: bool,
    /// Buffer éditable (contenu du résultat, modifiable).
    pub edit_buffer: Vec<String>,
    /// Ligne du curseur dans le buffer d'édition.
    pub edit_cursor_line: usize,
    /// Colonne du curseur dans le buffer d'édition.
    pub edit_cursor_col: usize,
}
```

Initialiser `is_editing: false`, `edit_buffer: Vec::new()`, `edit_cursor_line: 0`, `edit_cursor_col: 0`.

### 4. `src/state.rs` — Nouvelles actions

```rust
/// Entrer en mode édition dans le panneau résultat.
ConflictStartEditing,
/// Quitter le mode édition (Esc).
ConflictStopEditing,
/// Insérer un caractère en mode édition.
ConflictEditInsertChar(char),
/// Supprimer le caractère avant le curseur.
ConflictEditBackspace,
/// Supprimer le caractère sous le curseur.
ConflictEditDelete,
/// Déplacer le curseur en mode édition.
ConflictEditCursorUp,
ConflictEditCursorDown,
ConflictEditCursorLeft,
ConflictEditCursorRight,
/// Insérer une nouvelle ligne.
ConflictEditNewline,
```

### 5. `src/ui/input.rs` — Keybindings mode édition

Quand `state.conflicts_state.is_editing == true`, capturer **toutes les touches** comme du texte :

```rust
// Si en mode édition dans le résultat
if is_editing {
    return match key.code {
        KeyCode::Esc => Some(AppAction::ConflictStopEditing),
        KeyCode::Char(c) => Some(AppAction::ConflictEditInsertChar(c)),
        KeyCode::Backspace => Some(AppAction::ConflictEditBackspace),
        KeyCode::Delete => Some(AppAction::ConflictEditDelete),
        KeyCode::Enter => Some(AppAction::ConflictEditNewline),
        KeyCode::Up => Some(AppAction::ConflictEditCursorUp),
        KeyCode::Down => Some(AppAction::ConflictEditCursorDown),
        KeyCode::Left => Some(AppAction::ConflictEditCursorLeft),
        KeyCode::Right => Some(AppAction::ConflictEditCursorRight),
        _ => None,
    };
}
```

Pour entrer en mode édition, utiliser `i` (vim-like) ou `e` quand le focus est sur `ResultPanel` :

```rust
KeyCode::Char('i') | KeyCode::Char('e') => {
    if panel_focus == Some(ConflictPanelFocus::ResultPanel) {
        Some(AppAction::ConflictStartEditing)
    } else {
        None
    }
}
```

### 6. `src/event.rs` — Handlers d'édition

**`handle_conflict_start_editing()`** :
1. Générer le contenu résolu actuel via `generate_resolved_content()`.
2. Le copier dans `edit_buffer`.
3. Mettre `is_editing = true`, `edit_cursor_line = 0`, `edit_cursor_col = 0`.

**`handle_conflict_stop_editing()`** :
1. Mettre `is_editing = false`.
2. Garder le `edit_buffer` en mémoire (il sera utilisé lors de la validation).

**`handle_conflict_edit_insert_char(c)`** :
1. Insérer `c` dans `edit_buffer[edit_cursor_line]` à la position `edit_cursor_col`.
2. Incrémenter `edit_cursor_col`.

**`handle_conflict_edit_backspace()`** :
1. Si `edit_cursor_col > 0` : supprimer le caractère avant le curseur.
2. Si `edit_cursor_col == 0` et `edit_cursor_line > 0` : fusionner avec la ligne précédente.

**`handle_conflict_edit_newline()`** :
1. Couper la ligne courante à `edit_cursor_col`.
2. Insérer une nouvelle ligne avec le reste.
3. `edit_cursor_line += 1`, `edit_cursor_col = 0`.

**Curseurs** : mise à jour classique avec bornes min/max.

### 7. `src/ui/conflicts_view.rs` — Rendu en mode édition

Quand `is_editing` est actif, le panneau résultat :
- Affiche le contenu de `edit_buffer` au lieu du résultat généré.
- Affiche un curseur visuel (caractère inversé ou `|`) à la position `(edit_cursor_line, edit_cursor_col)`.
- Le titre du bloc devient `"Résultat [ÉDITION]"`.
- La bordure devient `Color::Magenta` pour signaler le mode édition.

```rust
let block = Block::default()
    .title(if state.is_editing { "Résultat [ÉDITION]" } else { "Résultat" })
    .borders(Borders::ALL)
    .border_style(if state.is_editing {
        Style::default().fg(Color::Magenta)
    } else if state.panel_focus == ConflictPanelFocus::ResultPanel {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    });
```

### 8. `src/event.rs` — Validation avec buffer édité

Modifier `handle_conflict_resolve_file()` : si `is_editing` et `edit_buffer` n'est pas vide, écrire directement le `edit_buffer` dans le fichier au lieu de passer par `resolve_file()` :

```rust
if conflicts_state.is_editing && !conflicts_state.edit_buffer.is_empty() {
    let content = conflicts_state.edit_buffer.join("\n");
    std::fs::write(&file_path, content)?;
    // git add
    let mut index = repo.index()?;
    index.add_path(Path::new(&file_path))?;
    index.write()?;
    // Marquer comme résolu
    conflicts_state.is_editing = false;
}
```

## Résultat attendu

- Les lignes du panneau résultat ont un **background coloré** :
  - Vert foncé pour les lignes venant de "ours"
  - Bleu foncé pour les lignes venant de "theirs"
  - Pas de background pour le contexte
  - Jaune gras pour les marqueurs de conflit non résolus
- Appuyer `i` ou `e` dans le panneau Résultat active le **mode édition**.
- En mode édition, on peut taper du texte, supprimer, ajouter des lignes.
- `Esc` quitte le mode édition.
- `Enter` (validation) prend en compte les modifications manuelles.
