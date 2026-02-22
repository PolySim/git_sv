# STEP-8 : Le flag `is_amending` est ignoré lors du commit

## Problème

Quand on utilise la commande "amend commit" (via `handle_amend_commit()` dans `handler/git.rs`), l'état `staging_state.is_amending = true` est correctement positionné, mais lors de la confirmation du commit, `handle_confirm_commit()` dans `handler/staging.rs` appelle toujours `create_commit()` au lieu de `amend_commit()`.

## Cause racine

`handle_confirm_commit()` ne vérifie pas le flag `is_amending` :

```rust
// handler/staging.rs:~110-125
fn handle_confirm_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging
        && !state.staging_state.commit_message.is_empty()
    {
        let message = state.staging_state.commit_message.clone();
        crate::git::commit::create_commit(&state.repo.repo, &message)?;  // ← Toujours create

        state.staging_state.is_committing = false;
        state.staging_state.commit_message.clear();
        state.staging_state.focus = StagingFocus::Unstaged;
        state.mark_dirty();
        refresh_staging(state)?;
    }
    Ok(())
}
```

Le champ `is_amending` est défini à `true` par `handle_amend_commit()` dans `git.rs:~160-174`, mais `handle_confirm_commit()` ne le vérifie jamais et ne le réinitialise pas non plus.

## Fichiers concernés

| Fichier | Ligne(s) | Problème |
|---|---|---|
| `src/handler/staging.rs` | `handle_confirm_commit()` ~110-125 | Ignore `is_amending` |
| `src/handler/git.rs` | `handle_amend_commit()` ~155-174 | Positionne correctement `is_amending` (OK) |

## Correction proposée

### Modifier `handle_confirm_commit()` dans `handler/staging.rs`

```rust
fn handle_confirm_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging
        && !state.staging_state.commit_message.is_empty()
    {
        let message = state.staging_state.commit_message.clone();

        if state.staging_state.is_amending {
            crate::git::commit::amend_commit(&state.repo.repo, &message)?;
            state.set_flash_message("Commit amendé ✓".to_string());
        } else {
            crate::git::commit::create_commit(&state.repo.repo, &message)?;
            state.set_flash_message("Commit créé ✓".to_string());
        }

        // Réinitialiser l'état du commit
        state.staging_state.is_committing = false;
        state.staging_state.is_amending = false;  // ← Important : réinitialiser
        state.staging_state.commit_message.clear();
        state.staging_state.cursor_position = 0;
        state.staging_state.focus = StagingFocus::Unstaged;

        state.mark_dirty();
        refresh_staging(state)?;
    }
    Ok(())
}
```

> **Note :** Vérifier que `crate::git::commit::amend_commit()` existe bien dans `src/git/commit.rs`. Si la fonction n'existe pas, elle devra être créée :
> ```rust
> pub fn amend_commit(repo: &Repository, message: &str) -> Result<()> {
>     let head = repo.head()?.peel_to_commit()?;
>     let tree_id = repo.index()?.write_tree()?;
>     let tree = repo.find_tree(tree_id)?;
>     let sig = repo.signature()?;
>     head.amend(Some("HEAD"), Some(&sig), Some(&sig), None, Some(message), Some(&tree))?;
>     Ok(())
> }
> ```

### Aussi réinitialiser `is_amending` dans `handle_cancel_commit()`

```rust
fn handle_cancel_commit(state: &mut AppState) -> Result<()> {
    if state.view_mode == ViewMode::Staging {
        state.staging_state.is_committing = false;
        state.staging_state.is_amending = false;  // ← Ajouter
        state.staging_state.commit_message.clear();
        state.staging_state.cursor_position = 0;
        state.staging_state.focus = StagingFocus::Unstaged;
    }
    Ok(())
}
```

## Vérification

1. `cargo build` compile
2. `cargo test` passe
3. Faire un commit normal via `c` → fonctionne comme avant
4. Lancer "amend commit" → le message du dernier commit est pré-rempli
5. Modifier le message et confirmer → le dernier commit est amendé (pas de nouveau commit créé)
6. Vérifier avec `git log` que le commit a bien été amendé
7. Annuler un amend avec `Esc` → le mode amend est réinitialisé, un futur `c` crée un nouveau commit
