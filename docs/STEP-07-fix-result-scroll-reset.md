# STEP-07 : Réinitialiser le scroll du panneau Résultat au changement de fichier

## Problème

Quand l'utilisateur change de fichier dans la liste des conflits, le scroll du panneau Résultat (`result_scroll`) conserve sa valeur précédente. Cela peut afficher une zone vide ou un contenu décalé pour le nouveau fichier.

## Fichiers à modifier

- `src/handler/conflict.rs` — Fonctions `handle_next_file` et `handle_previous_file`

## Corrections

### 1. Reset du scroll dans `handle_next_file` (~ligne 58)

Ajouter après le changement de `file_selected` :
```rust
conflicts.result_scroll = 0;
conflicts.ours_scroll = 0;
conflicts.theirs_scroll = 0;
conflicts.section_selected = 0;
conflicts.line_selected = 0;
```

### 2. Reset du scroll dans `handle_previous_file` (~ligne 68)

Même reset.

### 3. Borner `result_scroll` (amélioration)

Dans `handle_result_scroll_down`, ajouter une vérification que `result_scroll` ne dépasse pas le nombre total de lignes du contenu résolu :

```rust
fn handle_result_scroll_down(state: &mut AppState) -> Result<()> {
    if let Some(conflicts) = &mut state.conflicts_state {
        let total_lines = /* calculer le nombre de lignes du contenu résolu */;
        let visible_height = /* hauteur visible du panneau */;
        let max_scroll = total_lines.saturating_sub(visible_height);
        if conflicts.result_scroll < max_scroll {
            conflicts.result_scroll += 1;
        }
    }
    Ok(())
}
```

### 4. Vérifier aussi le reset lors d'un changement de mode

Quand on change de mode (File/Block/Line via `F`/`B`/`L`), réinitialiser `result_scroll` à 0.

## Vérification

```bash
cargo build
# Tester : scroller dans le résultat d'un fichier, puis changer de fichier
# Vérifier que le résultat du nouveau fichier commence en haut
```
