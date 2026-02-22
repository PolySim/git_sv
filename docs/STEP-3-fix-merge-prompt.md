# STEP-3 : Le merge (`m`) ne fait rien

## Problème

Presser `m` dans la vue Graph ou Branches ne fait rien. Le sélecteur de branches pour merge n'apparaît jamais.

## Cause racine

`handle_merge_prompt()` dans `handler/git.rs` et `handle_merge()` dans `handler/branch.rs` sont des **stubs vides** qui ne créent jamais le `MergePickerState`.

### Détail du flux cassé

1. **`input.rs`** mappe `m` vers `AppAction::MergePrompt` (Graph view, ligne ~226) ou `AppAction::Branch(BranchAction::Merge)` (Branches view, section Branches).

2. **`dispatcher.rs:172`** route `AppAction::MergePrompt` → `GitHandler` → `handle_merge_prompt()`.

3. **`handler/git.rs:~211-214`** :
   ```rust
   fn handle_merge_prompt(state: &mut AppState) -> Result<()> {
       // Ouvre le sélecteur de branches pour merge
       // Cette fonction nécessite une logique UI
       Ok(())
   }
   ```
   **Stub vide.** Ne crée jamais de `MergePickerState`.

4. **`handler/branch.rs:~103-106`** (pour la vue Branches) :
   ```rust
   fn handle_merge(_state: &mut AppState) -> Result<()> {
       // Ouvre le sélecteur de merge
       Ok(())
   }
   ```
   **Stub vide** aussi.

5. Le rendu dans `ui/mod.rs:~97-101` vérifie `state.merge_picker` :
   ```rust
   if let Some(ref picker) = state.merge_picker {
       if picker.is_active {
           merge_picker::render(frame, picker, &state.current_branch, frame.area());
       }
   }
   ```
   Mais `state.merge_picker` est toujours `None` puisque personne ne le crée.

6. La logique du merge picker une fois ouvert (navigation j/k, confirm, cancel) est **correctement implémentée** dans le dispatcher (lignes ~116-139). Le problème est uniquement l'ouverture.

## Fichiers concernés

| Fichier | Ligne(s) | Problème |
|---|---|---|
| `src/handler/git.rs` | `handle_merge_prompt()` ~211-214 | Stub vide |
| `src/handler/branch.rs` | `handle_merge()` ~103-106 | Stub vide |
| `src/state/view/merge_picker.rs` | `MergePickerState` | Structure définie mais jamais instanciée |

## Correction proposée

### 1. Implémenter `handle_merge_prompt()` dans `handler/git.rs`

```rust
fn handle_merge_prompt(state: &mut AppState) -> Result<()> {
    // Charger la liste des branches pour le merge picker
    match crate::git::branch::list_all_branches(&state.repo.repo) {
        Ok((local, remote)) => {
            let current = state.current_branch.clone().unwrap_or_default();

            // Construire la liste des branches (exclure la branche courante)
            let mut branch_names: Vec<String> = local
                .iter()
                .filter(|b| b.name != current)
                .map(|b| b.name.clone())
                .collect();

            // Ajouter les branches remote
            for b in &remote {
                branch_names.push(b.name.clone());
            }

            if branch_names.is_empty() {
                state.set_flash_message("Aucune autre branche disponible pour merge".to_string());
                return Ok(());
            }

            state.merge_picker = Some(crate::state::MergePickerState::new(branch_names));
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur chargement branches: {}", e));
        }
    }
    Ok(())
}
```

### 2. Implémenter `handle_merge()` dans `handler/branch.rs`

Ce handler devrait faire la même chose (ou déléguer) depuis la vue Branches :

```rust
fn handle_merge(state: &mut AppState) -> Result<()> {
    // Réutiliser la même logique que handle_merge_prompt
    match crate::git::branch::list_all_branches(&state.repo.repo) {
        Ok((local, remote)) => {
            let current = state.current_branch.clone().unwrap_or_default();

            let mut branch_names: Vec<String> = local
                .iter()
                .filter(|b| b.name != current)
                .map(|b| b.name.clone())
                .collect();

            for b in &remote {
                branch_names.push(b.name.clone());
            }

            if branch_names.is_empty() {
                state.set_flash_message("Aucune autre branche disponible pour merge".to_string());
                return Ok(());
            }

            state.merge_picker = Some(crate::state::MergePickerState::new(branch_names));
        }
        Err(e) => {
            state.set_flash_message(format!("Erreur: {}", e));
        }
    }
    Ok(())
}
```

> **Alternative :** Factoriser cette logique dans une fonction utilitaire partagée pour éviter la duplication.

## Vérification

1. `cargo build` compile
2. Lancer l'app en vue Graph, presser `m` → le picker de merge apparaît avec la liste des branches (sans la branche courante)
3. Naviguer avec `j`/`k`, confirmer avec `Enter` → le merge s'exécute
4. Annuler avec `Esc` → le picker se ferme
5. Aller en vue Branches (`3`), sélectionner une branche, presser `m` → le picker apparaît aussi
