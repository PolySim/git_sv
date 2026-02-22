# STEP-06 : Corriger le mode édition (visibilité conditionnelle + buffer vide)

## Problème

1. Le mode éditer (`i`/`e`) est accessible depuis n'importe quel panneau, mais ne devrait être proposé que quand le focus est sur le panneau Résultat
2. Quand activé, la zone d'édition est **vide** car `edit_buffer` n'est jamais initialisé avec le contenu résolu
3. Toutes les opérations d'édition (insertion, suppression, curseur, newline) sont des **stubs** vides

## Fichiers à modifier

- `src/ui/input.rs` — Restreindre l'activation au panneau Résultat (vérifier si déjà fait)
- `src/handler/conflict.rs` — Remplir `edit_buffer` à l'activation + implémenter les opérations d'édition
- `src/ui/conflicts_view.rs` — Vérifier le rendu de la help_bar (ne proposer `i`/`e` que pour ResultPanel)

## Corrections

### 1. Vérifier la restriction d'activation (`src/ui/input.rs`)

Selon l'analyse, `i`/`e` est déjà conditionné à `panel_focus == ResultPanel` dans `map_conflicts_key`. Vérifier que c'est bien le cas. Si oui, la help_bar doit aussi ne montrer le raccourci `i:Editer` que quand le focus est sur ResultPanel.

### 2. Remplir `edit_buffer` à l'activation (`src/handler/conflict.rs`, `handle_start_editing`)

```rust
fn handle_start_editing(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        // Générer le contenu résolu actuel
        let resolved_content = generate_resolved_content(&conflicts.all_files[conflicts.file_selected]);
        
        // Remplir le buffer d'édition avec les lignes du contenu résolu
        conflicts.edit_buffer = resolved_content
            .lines()
            .map(|l| l.to_string())
            .collect();
        
        // Positionner le curseur au début
        conflicts.edit_cursor_line = 0;
        conflicts.edit_cursor_col = 0;
        
        conflicts.is_editing = true;
    }
    Ok(())
}
```

### 3. Implémenter les opérations d'édition (`src/handler/conflict.rs`)

Chaque stub doit être implémenté :

- **`handle_edit_insert_char(state, c)`** : Insérer `c` à la position du curseur dans `edit_buffer[cursor_line]` à `cursor_col`, puis avancer `cursor_col`.
- **`handle_edit_backspace(state)`** : Si `cursor_col > 0`, supprimer le caractère avant le curseur. Si `cursor_col == 0` et `cursor_line > 0`, fusionner avec la ligne précédente.
- **`handle_edit_delete(state)`** : Supprimer le caractère sous le curseur. Si fin de ligne, fusionner avec la ligne suivante.
- **`handle_edit_cursor_up/down/left/right(state)`** : Déplacer le curseur dans le buffer avec bornes.
- **`handle_edit_newline(state)`** : Splitter la ligne courante en deux à la position du curseur, insérer une nouvelle ligne.

### 4. Sauvegarder les modifications à la confirmation

`handle_confirm_edit` doit écrire le contenu de `edit_buffer` sur le disque et mettre à jour l'index git.

## Vérification

```bash
cargo build
# Tester : se placer sur ResultPanel, appuyer 'i', vérifier que le contenu résolu s'affiche
# Tester la saisie de texte, backspace, navigation curseur, Enter pour nouvelle ligne
# Tester Esc pour quitter l'édition
```
