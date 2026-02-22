# STEP-6 : Implémenter les stubs restants dans les handlers

## Problème

Plusieurs handlers sont des stubs vides (no-op) suite à la refactorisation. Ces fonctions retournent `Ok(())` sans effectuer aucune action.

## Inventaire des stubs

### `handler/branch.rs`

| Fonction | Raccourci | Commentaire |
|---|---|---|
| `handle_create()` | `n` (Branches) | Devrait ouvrir l'input pour créer une branche |
| `handle_rename()` | `r` (Branches) | Devrait ouvrir l'input pour renommer |
| `handle_stash_save()` | `s` (Branches/Stashes) | Devrait ouvrir l'input pour le message du stash |
| `handle_worktree_create()` | `n` (Branches/Worktrees) | Devrait ouvrir l'input pour créer un worktree |

### `handler/staging.rs`

| Fonction | Raccourci | Commentaire |
|---|---|---|
| `handle_discard_file()` | `d` (Staging) | Devrait ouvrir une confirmation puis discard |
| `handle_discard_all()` | `D` (Staging) | Devrait ouvrir une confirmation puis discard all |
| `handle_stash_selected_file()` | — | Placeholder |
| `handle_stash_unstaged_files()` | — | Placeholder |

### `handler/conflict.rs`

| Fonction | Commentaire |
|---|---|
| `handle_next_section()` | Navigation section dans conflits |
| `handle_accept_ours_block()` | Accepter "ours" au niveau bloc |
| `handle_accept_theirs_block()` | Accepter "theirs" au niveau bloc |
| `handle_accept_both()` | Accepter les deux |
| `handle_toggle_line()` | Toggle ligne dans résolution ligne |
| `handle_edit_*()` (8 fonctions) | Éditeur inline de résolution |
| `handle_enter_resolve()` | Entrer en mode résolution |

## Fichiers concernés

| Fichier | Stubs |
|---|---|
| `src/handler/branch.rs` | 4 fonctions |
| `src/handler/staging.rs` | 4 fonctions |
| `src/handler/conflict.rs` | ~14 fonctions |

## Corrections proposées

### 1. `handle_create()` dans `handler/branch.rs`

```rust
fn handle_create(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateBranch);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}
```

### 2. `handle_rename()` dans `handler/branch.rs`

```rust
fn handle_rename(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        if let Some(branch) = state.branches_view_state.selected_branch() {
            let current_name = branch.name.clone();
            state.branches_view_state.focus = BranchesFocus::Input;
            state.branches_view_state.input_action = Some(InputAction::RenameBranch);
            state.branches_view_state.input_text = current_name;
            state.branches_view_state.input_cursor = state.branches_view_state.input_text.len();
        }
    }
    Ok(())
}
```

### 3. `handle_stash_save()` dans `handler/branch.rs`

```rust
fn handle_stash_save(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::SaveStash);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}
```

### 4. `handle_worktree_create()` dans `handler/branch.rs`

```rust
fn handle_worktree_create(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Branches {
        state.branches_view_state.focus = BranchesFocus::Input;
        state.branches_view_state.input_action = Some(InputAction::CreateWorktree);
        state.branches_view_state.input_text.clear();
        state.branches_view_state.input_cursor = 0;
    }
    Ok(())
}
```

### 5. Implémenter `ConfirmInput` / `CancelInput` dans `handler/branch.rs`

Actuellement les variantes `BranchAction::ConfirmInput` et `BranchAction::CancelInput` retournent juste `Ok(())` dans le match du handler. Il faut les implémenter :

```rust
BranchAction::ConfirmInput => handle_confirm_input(ctx.state),
BranchAction::CancelInput => handle_cancel_input(ctx.state),
```

```rust
fn handle_confirm_input(state: &mut AppState) -> Result<()> {
    let input = state.branches_view_state.input_text.trim().to_string();
    if input.is_empty() {
        state.branches_view_state.focus = BranchesFocus::List;
        state.branches_view_state.input_action = None;
        return Ok(());
    }

    match state.branches_view_state.input_action {
        Some(InputAction::CreateBranch) => {
            match crate::git::branch::create_branch(&state.repo.repo, &input) {
                Ok(_) => {
                    state.set_flash_message(format!("Branche '{}' créée ✓", input));
                    state.mark_dirty();
                }
                Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
            }
        }
        Some(InputAction::RenameBranch) => {
            if let Some(branch) = state.branches_view_state.selected_branch() {
                let old_name = branch.name.clone();
                match crate::git::branch::rename_branch(&state.repo.repo, &old_name, &input) {
                    Ok(_) => {
                        state.set_flash_message(format!("Branche renommée → '{}' ✓", input));
                        state.mark_dirty();
                    }
                    Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
                }
            }
        }
        Some(InputAction::SaveStash) => {
            match crate::git::stash::create_stash(&mut state.repo.repo, Some(&input)) {
                Ok(_) => {
                    state.set_flash_message(format!("Stash créé: {} ✓", input));
                    state.mark_dirty();
                }
                Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
            }
        }
        Some(InputAction::CreateWorktree) => {
            // Le format attendu est "nom chemin [branche]"
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let path = parts[1];
                let branch = parts.get(2).copied();
                match crate::git::worktree::create_worktree(&state.repo.repo, path, branch) {
                    Ok(_) => {
                        state.set_flash_message(format!("Worktree '{}' créé ✓", name));
                        state.mark_dirty();
                    }
                    Err(e) => state.set_flash_message(format!("Erreur: {}", e)),
                }
            } else {
                state.set_flash_message("Format: nom chemin [branche]".to_string());
            }
        }
        None => {}
    }

    state.branches_view_state.focus = BranchesFocus::List;
    state.branches_view_state.input_action = None;
    state.branches_view_state.input_text.clear();
    state.branches_view_state.input_cursor = 0;
    Ok(())
}

fn handle_cancel_input(state: &mut AppState) -> Result<()> {
    state.branches_view_state.focus = BranchesFocus::List;
    state.branches_view_state.input_action = None;
    state.branches_view_state.input_text.clear();
    state.branches_view_state.input_cursor = 0;
    Ok(())
}
```

> **Note :** Vérifier les signatures exactes des fonctions `crate::git::branch::create_branch`, `rename_branch`, `crate::git::stash::create_stash`, `crate::git::worktree::create_worktree` dans le module git avant d'implémenter.

### 6. `handle_discard_file()` et `handle_discard_all()` dans `handler/staging.rs`

Ces fonctions devraient ouvrir le dialogue de confirmation :

```rust
fn handle_discard_file(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Staging {
        if let Some(file) = state
            .staging_state
            .unstaged_files()
            .get(state.staging_state.unstaged_selected())
        {
            let path = file.path.clone();
            state.pending_confirmation = Some(ConfirmAction::DiscardFile(path));
        }
    }
    Ok(())
}

fn handle_discard_all(state: &mut AppState) -> Result<()> {
    use crate::ui::confirm_dialog::ConfirmAction;

    if state.view_mode == ViewMode::Staging {
        state.pending_confirmation = Some(ConfirmAction::DiscardAll);
    }
    Ok(())
}
```

### 7. Handlers de conflits (optionnel — peut être reporté)

Les 14 stubs dans `handler/conflict.rs` concernent la résolution de conflits avancée (bloc, ligne, édition inline). L'implémentation est complexe et nécessite une compréhension approfondie de la structure `ConflictsState`. Ils peuvent être traités dans un STEP séparé dédié à la fonctionnalité de résolution de conflits.

## Priorité de correction

1. **Haute** : `handle_discard_file/all` (staging) — empêche le discard de fichiers
2. **Haute** : `handle_create` + `handle_confirm_input` (branch) — empêche la création de branches
3. **Moyenne** : `handle_rename`, `handle_stash_save`, `handle_worktree_create` (branch)
4. **Basse** : `handle_stash_selected_file`, `handle_stash_unstaged_files` (staging) — fonctionnalités avancées
5. **Basse** : Tous les handlers de conflits — fonctionnalité complexe à part

## Vérification

Pour chaque handler corrigé :
1. `cargo build` compile
2. `cargo test` passe
3. Tester le raccourci correspondant dans l'app et vérifier que l'action s'exécute
