# STEP-04 — Corriger la résolution de conflits par bloc et par ligne

## Problème

Quand on est en mode bloc et qu'on finit de sélectionner tous les blocs, le fichier n'est pas noté comme résolu ni synchronisé sur le disque. Le même souci existe pour la sélection par ligne. La résolution par fichier entier fonctionne correctement.

### Cause racine

**Les résolutions bloc/ligne ne sont jamais écrites sur le disque ni dans l'index git.** Voici le détail :

1. **Mode bloc** : Les handlers `handle_accept_ours_block`, `handle_accept_theirs_block`, `handle_accept_both` ne font que modifier `conflict.resolution` **en mémoire**. Aucune fonction n'assemble ensuite le contenu résolu et ne l'écrit sur le disque.

2. **Mode ligne** : Les handlers `ToggleLine` modifient `conflict.line_level_resolution.ours_lines_included[i]` / `theirs_lines_included[i]` **en mémoire seulement**. La fonction `generate_resolved_content()` existe et produit le bon résultat, mais elle n'est appelée que pour l'affichage dans le panneau résultat.

3. **`MarkResolved`** ne fait que mettre `file.is_resolved = true` en mémoire — il n'écrit rien sur le disque et ne met pas à jour l'index git. `finalize_merge()` échouera car l'index contient toujours des entrées de conflit (stages 1/2/3).

4. **Le mode éditeur** (`StartEditing` → `ConfirmEdit`) est le seul chemin qui écrit sur le disque et met à jour l'index, mais `handle_start_editing` ignore les résolutions par ligne (il utilise `conflict.resolution` au niveau bloc, pas `line_level_resolution`).

## Fichiers concernés

| Fichier | Modification |
|---------|-------------|
| `src/handler/conflict.rs` | Ajouter la logique d'écriture disque + index pour bloc/ligne |
| `src/git/conflict.rs` | Ajouter une fonction `apply_block_line_resolution()` ou similaire |
| `src/state/view/conflicts.rs` | Éventuellement ajouter un helper pour vérifier si tous les blocs sont résolus |

## Corrections à apporter

### 1. Ajouter une fonction `apply_resolved_content()` dans `git/conflict.rs`

Cette fonction doit :
- Prendre un `MergeFile` avec ses `ConflictSection`s résolues (bloc ou ligne)
- Appeler `generate_resolved_content()` pour produire le contenu final
- Écrire le contenu sur le disque (`std::fs::write`)
- Supprimer les entrées de conflit de l'index git (stages 1/2/3)
- Ajouter le fichier résolu à l'index (stage 0)

```rust
pub fn apply_resolved_content(repo: &Repository, file: &MergeFile) -> Result<()> {
    let content = generate_resolved_content(&file.conflicts);
    std::fs::write(&file.path, &content)?;

    // Mettre à jour l'index git
    let mut index = repo.index()?;
    index.remove_path(Path::new(&file.path), 0)?; // stages 1/2/3
    index.add_path(Path::new(&file.path))?;        // stage 0
    index.write()?;
    Ok(())
}
```

### 2. Auto-détecter quand tous les blocs/lignes sont résolus

Ajouter un helper dans `conflicts.rs` (state) ou `conflict.rs` (git) :

```rust
pub fn all_sections_resolved(file: &MergeFile) -> bool {
    file.conflicts.iter().all(|c| c.resolution.is_some())
}
```

### 3. Déclencher l'écriture quand tous les blocs sont résolus

Dans `handler/conflict.rs`, après chaque `handle_accept_ours_block` / `handle_accept_theirs_block` / `handle_accept_both` :

```rust
// Vérifier si tous les blocs sont résolus
if all_sections_resolved(&file) {
    apply_resolved_content(&ctx.repo, &file)?;
    file.is_resolved = true;
    // Avancer au prochain fichier non résolu
}
```

Alternativement, ajouter une action explicite `ApplyBlockResolution` que l'utilisateur déclenche manuellement (ex: touche `Enter` ou `r` sur le panneau résultat).

### 4. Même logique pour le mode ligne

Après chaque `ToggleLine`, vérifier si l'utilisateur a fait un choix pour chaque ligne de chaque section. Si oui, proposer/appliquer la résolution. Ou bien ajouter une action manuelle `ApplyLineResolution`.

### 5. Corriger `handle_start_editing` pour le mode ligne

`handle_start_editing` doit utiliser `generate_resolved_content()` (qui prend en compte `line_level_resolution`) au lieu de reconstruire le buffer manuellement à partir de `conflict.resolution` :

```rust
fn handle_start_editing(state: &mut AppState) -> Result<()> {
    // ...
    let content = generate_resolved_content(&file.conflicts);
    state.conflicts.edit_buffer = content.lines().map(|l| l.to_string()).collect();
    // ...
}
```

### 6. Corriger `MarkResolved` pour écrire sur le disque

`handle_mark_resolved` doit non seulement mettre `is_resolved = true` mais aussi écrire le contenu résolu sur le disque et mettre à jour l'index git (en appelant `apply_resolved_content`).

## Vérification

- `cargo build` compile
- `cargo test` passe
- `cargo clippy` sans warning
- Tester mode bloc : sélectionner ours/theirs pour chaque section → le fichier est écrit sur le disque et marqué résolu dans l'index git
- Tester mode ligne : toggler les lignes → le contenu résolu est écrit et l'index est mis à jour
- Tester `finalize_merge` après résolutions bloc/ligne → le merge commit est créé correctement
- Tester le mode éditeur après des sélections en mode ligne → le buffer contient le bon contenu
