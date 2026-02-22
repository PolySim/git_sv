# STEP-10 : Conflict — Sauvegarder les modifications manuelles de l'éditeur

## Problème

Quand l'utilisateur édite manuellement le résultat dans le panneau Résultat (mode édition via `i`/`e`), les modifications ne sont **jamais sauvegardées**. En quittant l'édition avec `Esc`, le buffer est simplement abandonné.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/handler/conflict.rs` | 323-378 | `handle_confirm_edit` — écrit le buffer sur disque + git add (**fonctionne mais jamais appelé**) |
| `src/handler/conflict.rs` | 380-385 | `handle_cancel_edit` — met `is_editing = false` (pas de sauvegarde) |
| `src/handler/conflict.rs` | 656-659 | `handle_stop_editing` — idem, `is_editing = false` sans sauvegarde |
| `src/ui/input.rs` | 323-334 | Keybindings en mode édition : `Esc` → `ConflictStopEditing` |
| `src/handler/dispatcher.rs` | - | **Aucun routing pour `ConflictAction::ConfirmEdit`** |

## Analyse

Le bug est **un binding manquant**. La logique de sauvegarde existe et est complète dans `handle_confirm_edit` :
- Écrit `edit_buffer.join("\n")` dans le fichier
- Fait `git add` sur le fichier via l'index
- Marque le fichier comme résolu
- Met `is_editing = false`

Mais cette fonction n'est **jamais appelée** car :
1. `ConflictAction::ConfirmEdit` est bien défini dans l'enum
2. `ConflictAction::ConfirmEdit` est routé dans le handler (`conflict.rs:11`)
3. **MAIS** il n'y a aucune touche mappée vers `AppAction::ConflictConfirmEdit` ou similaire dans `map_conflicts_key`

En mode édition, les seules touches mappées sont :
- `Esc` → `ConflictStopEditing` (quitte sans sauvegarder)
- Touches de texte (insert, backspace, delete, curseur, newline)

Il manque un binding pour **sauvegarder** (ex: `Ctrl+S` ou un autre raccourci).

## Solution proposée

### 1. Ajouter une action `AppAction::ConflictConfirmEdit`

**Vérifier si l'action existe déjà** dans l'enum `AppAction`. Si non, l'ajouter.

### 2. Ajouter le keybinding en mode édition

**Modifier `src/ui/input.rs`** — dans le bloc `if is_editing` de `map_conflicts_key` :

```rust
if is_editing {
    return match key.code {
        KeyCode::Esc => Some(AppAction::ConflictStopEditing),
        // ← AJOUTER : Sauvegarder et quitter l'édition
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::ConflictConfirmEdit)
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::ConflictConfirmEdit)
        }
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

**Note** : `Ctrl+Enter` sauvegarde et marque comme résolu, `Enter` simple insère une nouvelle ligne.

### 3. Router l'action dans le dispatcher

**Modifier `src/handler/dispatcher.rs`** — ajouter dans la section "Conflict legacy" :

```rust
AppAction::ConflictConfirmEdit => self.conflict.handle(&mut ctx, ConflictAction::ConfirmEdit),
```

### 4. Mettre à jour la help bar en mode édition

**Modifier `src/ui/conflicts_view.rs`** — `build_help_bar` (`conflicts_view.rs:113-115`) :

```rust
if state.is_editing {
    "Esc:Annuler  Ctrl+S:Sauvegarder  ↑↓←→:Curseur  Enter:Nouvelle ligne  Backspace:Suppr"
}
```

## Ordre d'implémentation

1. Ajouter `AppAction::ConflictConfirmEdit` dans l'enum `AppAction` (si absent)
2. Ajouter le keybinding `Ctrl+S` et `Ctrl+Enter` dans le bloc `is_editing` de `map_conflicts_key`
3. Ajouter le routing dans `dispatcher.rs`
4. Mettre à jour la help bar pour indiquer le raccourci de sauvegarde
5. Tester : éditer le résultat → `Ctrl+S` → vérifier que le fichier est écrit sur disque et marqué résolu

## Critère de validation

- `Ctrl+S` en mode édition sauvegarde les modifications sur disque
- Le fichier est ajouté à l'index git (`git add`)
- Le fichier est marqué comme résolu dans l'état
- `Esc` quitte toujours sans sauvegarder (annulation)
- La help bar affiche le raccourci de sauvegarde
- Un message flash confirme la sauvegarde
