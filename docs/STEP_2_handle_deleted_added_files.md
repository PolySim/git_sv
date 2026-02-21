# STEP 2 - Gestion des fichiers supprimés et ajoutés lors d'un merge

## Problème

Actuellement, `list_conflict_files()` (dans `src/git/conflict.rs` ligne ~137) ne gère que les conflits où `conflict.our` existe (fichier modifié des deux côtés). Les cas suivants ne sont pas gérés :

- **Fichier supprimé dans une branche, modifié dans l'autre** (`conflict.our` est `Some`, mais `conflict.their` est `None`, ou inversement)
- **Fichier ajouté dans les deux branches** avec des contenus différents
- **Fichier supprimé dans les deux branches** (ne devrait pas générer de conflit mais doit être géré proprement)

De plus, `list_all_merge_files()` (ligne ~349) liste tous les fichiers de l'index, mais ne détecte pas les fichiers supprimés dans l'une des branches.

## Fichiers à modifier

| Fichier | Rôle |
|---------|------|
| `src/git/conflict.rs` | `list_conflict_files()`, `list_all_merge_files()`, structures de données |
| `src/state.rs` | `MergeFile` / `ConflictFile` : ajouter le type de conflit |
| `src/ui/conflicts_view.rs` | Affichage des icônes et des panneaux pour les fichiers spéciaux |
| `src/event.rs` | Handlers de résolution pour les cas supprimé/ajouté |

## Modifications détaillées

### 1. `src/git/conflict.rs` — Structures de données

Ajouter un enum pour le type de conflit :

```rust
/// Type de conflit sur un fichier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictType {
    /// Conflit classique : modifié des deux côtés.
    BothModified,
    /// Supprimé dans ours, modifié dans theirs.
    DeletedByUs,
    /// Modifié dans ours, supprimé dans theirs.
    DeletedByThem,
    /// Ajouté dans les deux branches avec des contenus différents.
    BothAdded,
}
```

Ajouter le champ `conflict_type` dans `MergeFile` :

```rust
pub struct MergeFile {
    pub path: String,
    pub has_conflicts: bool,
    pub conflicts: Vec<ConflictSection>,
    pub is_resolved: bool,
    pub conflict_type: Option<ConflictType>,  // None si pas de conflit
}
```

### 2. `src/git/conflict.rs` — `list_conflict_files()`

Modifier la boucle des conflits (ligne ~146) pour gérer tous les cas :

```rust
for conflict in conflicts {
    let conflict = conflict.map_err(/* ... */)?;

    let (path, conflict_type) = match (&conflict.our, &conflict.their, &conflict.ancestor) {
        // Cas classique : modifié des deux côtés
        (Some(ours), Some(_theirs), _) => {
            let p = std::str::from_utf8(&ours.path)?.to_string();
            (p, ConflictType::BothModified)
        }
        // Supprimé dans ours, modifié dans theirs
        (None, Some(theirs), _) => {
            let p = std::str::from_utf8(&theirs.path)?.to_string();
            (p, ConflictType::DeletedByUs)
        }
        // Modifié dans ours, supprimé dans theirs
        (Some(ours), None, _) => {
            let p = std::str::from_utf8(&ours.path)?.to_string();
            (p, ConflictType::DeletedByThem)
        }
        // Pas d'ancêtre commun, ajouté des deux côtés
        (Some(ours), Some(_theirs), None) => {
            let p = std::str::from_utf8(&ours.path)?.to_string();
            (p, ConflictType::BothAdded)
        }
        _ => continue, // Cas impossible en théorie
    };

    // Pour les fichiers supprimés, pas de parsing de marqueurs de conflit
    // mais une "pseudo-section" pour permettre la résolution
    let sections = match conflict_type {
        ConflictType::BothModified | ConflictType::BothAdded => {
            parse_conflict_file(&path)?
        }
        ConflictType::DeletedByUs => {
            // Le fichier n'existe pas en local, lire le contenu depuis theirs
            vec![ConflictSection {
                context_before: vec![],
                ours: vec![],  // Supprimé
                theirs: read_blob_content(repo, &conflict.their.unwrap())?,
                context_after: vec![],
                resolution: None,
                line_resolutions: vec![],
            }]
        }
        ConflictType::DeletedByThem => {
            // Le fichier existe en local, theirs est vide
            vec![ConflictSection {
                context_before: vec![],
                ours: read_file_lines(&path)?,
                theirs: vec![],  // Supprimé
                context_after: vec![],
                resolution: None,
                line_resolutions: vec![],
            }]
        }
    };
    // ...
}
```

### 3. `src/git/conflict.rs` — Fonctions utilitaires

Ajouter deux fonctions helper :

```rust
/// Lit le contenu d'un blob depuis l'index git (pour les fichiers supprimés localement).
fn read_blob_content(repo: &Repository, entry: &git2::IndexEntry) -> Result<Vec<String>> {
    let blob = repo.find_blob(entry.id).map_err(/* ... */)?;
    let content = std::str::from_utf8(blob.content()).map_err(/* ... */)?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}

/// Lit les lignes d'un fichier local.
fn read_file_lines(path: &str) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path).map_err(/* ... */)?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}
```

### 4. `src/ui/conflicts_view.rs` — Affichage

Modifier `render_files_panel()` (ligne ~116) pour afficher des icônes spécifiques :

```
✗ D← fichier.txt    (DeletedByUs — supprimé chez nous)
✗ D→ fichier.txt    (DeletedByThem — supprimé chez eux)
✗ A+ fichier.txt    (BothAdded — ajouté des deux côtés)
✗    fichier.txt    (BothModified — comportement actuel)
```

Modifier `render_ours_theirs_panels()` pour afficher un message clair quand un côté est "supprimé" :

- Si `DeletedByUs` : panneau Ours affiche `"[Fichier supprimé]"` en rouge
- Si `DeletedByThem` : panneau Theirs affiche `"[Fichier supprimé]"` en rouge

La résolution pour ces cas spéciaux :
- `o` (ours) sur `DeletedByUs` → accepter la suppression (supprimer le fichier)
- `t` (theirs) sur `DeletedByUs` → garder le fichier (version theirs)
- `o` (ours) sur `DeletedByThem` → garder le fichier (version ours)
- `t` (theirs) sur `DeletedByThem` → accepter la suppression

### 5. `src/event.rs` — Résolution des fichiers supprimés

Modifier `handle_conflict_resolve_file()` (ligne ~2329) pour gérer la résolution de fichiers supprimés :

- Si résolution = "garder la suppression" → `git rm` le fichier de l'index
- Si résolution = "garder le fichier" → écrire le contenu et `git add`

### 6. `src/git/conflict.rs` — `resolve_file()`

Étendre `resolve_file()` (ligne ~179) pour gérer les cas `DeletedByUs` et `DeletedByThem`. Quand la résolution implique une suppression :

```rust
// Si la résolution est de supprimer le fichier
if should_delete {
    std::fs::remove_file(&file.path).ok(); // OK si déjà absent
    let mut index = repo.index()?;
    index.remove_path(Path::new(&file.path))?;
    index.write()?;
    return Ok(());
}
```

## Résultat attendu

- Les fichiers supprimés/ajoutés apparaissent dans la liste des conflits avec une icône distincte.
- L'utilisateur peut choisir "garder" ou "supprimer" via `o`/`t`.
- Les panneaux ours/theirs affichent clairement quand un côté est supprimé.
- La résolution écrit/supprime correctement le fichier sur disque.
