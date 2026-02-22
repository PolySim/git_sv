# STEP-03 : Corriger la résolution de conflit fichier (keep file) + rafraîchissement UI

## Problème

Quand l'utilisateur choisit de garder un fichier (ours/theirs au niveau fichier) :
1. Le fichier est bien modifié sur le disque et dans l'index git
2. Mais l'état en mémoire (`conflicts.all_files`) n'est **pas mis à jour** → l'UI continue d'afficher le fichier comme non résolu
3. `mark_dirty()` n'est **pas appelé** → pas de rafraîchissement automatique de l'état de l'application

## Fichiers à modifier

- `src/handler/conflict.rs` — Fonctions `handle_accept_ours_file` et `handle_accept_theirs_file`

## Corrections

### 1. Mettre à jour l'état en mémoire après résolution (~lignes 103-132)

Après l'appel réussi à `resolve_file_with_strategy()`, il faut :

```rust
// Après résolution réussie sur disque/index, mettre à jour l'état en mémoire
if let Some(conflicts) = &mut state.conflicts_state {
    if let Some(file) = conflicts.all_files.get_mut(conflicts.file_selected) {
        file.is_resolved = true;
        // Mettre à jour la résolution de toutes les sections
        for section in &mut file.sections {
            section.resolution = Some(strategy); // Ours ou Theirs selon le cas
        }
    }
}
```

### 2. Appeler `mark_dirty()` après résolution

Ajouter `state.mark_dirty()` après la résolution réussie pour déclencher un rafraîchissement de l'état global (status, staging, etc.).

### 3. Vérifier le binding clavier pour `ConflictResolveFile` / `MarkResolved`

Actuellement, l'action `ConflictResolveFile` (qui déclenche `MarkResolved`) n'a **aucun binding clavier**. Ajouter un binding (par ex. `r` dans le FileList panel) pour permettre de marquer manuellement un fichier comme résolu.

### 4. Avancer automatiquement au fichier suivant (amélioration UX)

Après résolution d'un fichier, avancer `file_selected` au prochain fichier non résolu si disponible.

## Vérification

```bash
cargo build
# Tester : résoudre un fichier avec 'o' ou 't', vérifier que l'UI se met à jour
# Vérifier que le fichier apparaît comme résolu dans la liste
```
