# STEP-05 — Corriger la résolution de conflits pour les fichiers supprimés

## Problème

Quand un fichier est supprimé (conflit `DeletedByUs` ou `DeletedByThem`), on ne peut pas sélectionner la version supprimée ni celle à conserver. L'erreur est :

```
Erreur: Impossible de lire le fichier 'apps/ses/src/app/[lng]/offer/components/AI/AIWritten.tsx':
No such file or directory (os error 2)
```

### Cause racine

Les handlers `AcceptOursFile` et `AcceptTheirsFile` appellent `resolve_file_with_strategy()` qui appelle `parse_conflict_file(path)` → `std::fs::read_to_string(path)`. Pour un conflit `DeletedByUs`, le fichier **n'existe pas sur le disque** (il a été supprimé côté "ours"). L'appel `read_to_string` échoue avec l'erreur OS.

De plus :

1. **`resolve_file_with_strategy()` hardcode `ConflictType::BothModified`**, quel que soit le type réel du conflit. Elle ne sait pas gérer les conflits de suppression.

2. **`resolve_special_file()` existe et gère correctement les conflits de suppression** (en lisant le contenu depuis les blobs git, pas depuis le disque), mais **elle n'est jamais appelée** depuis aucun handler.

3. Pour `DeletedByThem`, le fichier existe sur le disque mais la version "theirs" (supprimée) est un vecteur vide dans la structure de données. Le handler essaie quand même de parser des marqueurs de conflit dans le fichier, ce qui échoue silencieusement.

## Fichiers concernés

| Fichier | Modification |
|---------|-------------|
| `src/handler/conflict.rs` | Brancher vers `resolve_special_file()` pour les conflits de suppression |
| `src/git/conflict.rs` | Vérifier que `resolve_special_file()` gère bien tous les cas |

## Corrections à apporter

### 1. Détecter le type de conflit avant d'appeler la résolution

Dans `handler/conflict.rs`, `handle_accept_ours_file` et `handle_accept_theirs_file` :

```rust
fn handle_accept_ours_file(state: &mut AppState, ctx: &HandlerContext) -> Result<()> {
    let file = &state.conflicts.all_files[state.conflicts.file_selected];

    match file.conflict_type {
        ConflictType::DeletedByUs | ConflictType::DeletedByThem => {
            // Utiliser resolve_special_file pour les conflits de suppression
            let strategy = ConflictResolution::Ours;
            resolve_special_file(&ctx.repo, file, strategy)?;
        }
        ConflictType::BothModified | ConflictType::BothAdded => {
            // Chemin existant pour les fichiers avec marqueurs de conflit
            resolve_file_with_strategy(&ctx.repo, &file.path, ConflictResolution::Ours)?;
        }
    }

    file.is_resolved = true;
    // Avancer au prochain fichier non résolu
    // ...
}
```

### 2. Vérifier/corriger `resolve_special_file()`

S'assurer que `resolve_special_file()` dans `git/conflict.rs` gère correctement :

- **`DeletedByUs` + stratégie `Ours`** : supprimer le fichier (le garder supprimé), retirer de l'index
- **`DeletedByUs` + stratégie `Theirs`** : écrire le contenu du blob "theirs" sur le disque, ajouter à l'index
- **`DeletedByThem` + stratégie `Ours`** : garder le fichier existant, l'ajouter à l'index
- **`DeletedByThem` + stratégie `Theirs`** : supprimer le fichier du disque, retirer de l'index

Chaque cas doit nettoyer les entrées de conflit de l'index (stages 1/2/3) et écrire l'index.

### 3. Adapter l'UI du panneau conflits pour les fichiers supprimés

Dans le panneau conflits, les fichiers supprimés devraient afficher un message clair :

- Panneau "Ours" pour `DeletedByUs` : afficher `[Fichier supprimé]` ou le contenu du blob si disponible côté ours
- Panneau "Theirs" pour `DeletedByThem` : afficher `[Fichier supprimé]`

Les raccourcis d'aide devraient indiquer clairement :
- `o` : Garder la version ours (conserver / supprimer selon le cas)
- `t` : Garder la version theirs (conserver / supprimer selon le cas)

### 4. Désactiver les modes bloc/ligne pour les conflits de suppression

Les modes bloc et ligne n'ont pas de sens pour les conflits de suppression (il n'y a pas de sections de conflit à naviguer). Quand le fichier sélectionné est un conflit de type `DeletedByUs` ou `DeletedByThem`, forcer le mode fichier et empêcher le passage en mode bloc/ligne.

## Vérification

- `cargo build` compile
- `cargo test` passe
- `cargo clippy` sans warning
- Tester avec un conflit `DeletedByUs` : sélectionner "ours" (accepter la suppression) → pas d'erreur, fichier retiré de l'index
- Tester avec un conflit `DeletedByUs` : sélectionner "theirs" (restaurer le fichier) → le fichier est écrit depuis le blob git
- Tester avec un conflit `DeletedByThem` : sélectionner "theirs" (accepter la suppression) → fichier supprimé du disque
- Tester avec un conflit `DeletedByThem` : sélectionner "ours" (garder le fichier) → fichier ajouté à l'index
- Vérifier que `finalize_merge` fonctionne après résolution de conflits de suppression
