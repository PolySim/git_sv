# STEP-11 : Corriger l'erreur de commit "not fully merged index"

## Problème

Après avoir résolu tous les conflits et tenté de finaliser le merge, l'erreur suivante apparaît :

```
Error: Erreur git : cannot create a tree from a not fully merged index.; class=Index (10); code=Unmerged (-10)
```

Cela signifie que `index.write_tree()` échoue car l'index git contient encore des entrées de conflit (stages > 0) même après que l'utilisateur a résolu les conflits dans l'UI.

## Cause racine

La fonction `resolve_file()` dans `src/git/conflict.rs` :
1. Écrit le contenu résolu sur le disque
2. Fait `git add` (ajoute le fichier à l'index)

Mais l'étape de `git add` ne supprime pas correctement les entrées de conflit de l'index. Les entrées aux stages 1 (ancestor), 2 (ours) et 3 (theirs) persistent dans l'index git, et `index.write_tree()` refuse de créer un tree tant que ces entrées existent.

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/git/conflict.rs` | `resolve_file()` (~302-403) | S'assurer que les entrées de conflit sont nettoyées de l'index |
| `src/git/conflict.rs` | `resolve_special_file()` (~522-640) | Même correction |
| `src/git/conflict.rs` | `finalize_merge()` (~421-492) | Ajouter une vérification/nettoyage avant `write_tree()` |

## Détail des modifications

### 1. `src/git/conflict.rs` — Corriger `resolve_file()`

Après avoir écrit le fichier résolu sur le disque, il faut **explicitement** supprimer les entrées de conflit de l'index et ajouter l'entrée normale (stage 0) :

```rust
fn resolve_file(repo: &Repository, file: &ConflictFile, resolved_content: &str) -> Result<()> {
    let file_path = &file.path;
    
    // 1. Écrire le contenu résolu sur le disque
    let full_path = repo.workdir().unwrap().join(file_path);
    std::fs::write(&full_path, resolved_content)?;
    
    // 2. Supprimer les entrées de conflit de l'index
    let mut index = repo.index()?;
    
    // Supprimer explicitement les entrées aux stages 1, 2, 3
    // (les entrées de conflit)
    let _ = index.remove_path(Path::new(file_path)); // Supprime toutes les entrées (tous stages)
    
    // 3. Ré-ajouter le fichier à l'index (stage 0 = normal)
    index.add_path(Path::new(file_path))?;
    
    // 4. Écrire l'index sur le disque
    index.write()?;
    
    Ok(())
}
```

**Point clé** : `index.add_path()` seul ne suffit pas toujours à supprimer les entrées des stages supérieurs. Il faut d'abord faire `index.remove_path()` pour nettoyer toutes les entrées, puis `index.add_path()` pour ajouter l'entrée normale.

### 2. Alternative : utiliser `index.conflict_remove()`

La bibliothèque `git2` offre une méthode dédiée :

```rust
// Supprimer le conflit pour ce fichier
index.conflict_remove(Path::new(file_path))?;

// Ajouter le fichier résolu
index.add_path(Path::new(file_path))?;

// Écrire l'index
index.write()?;
```

`conflict_remove()` supprime spécifiquement les entrées des stages 1/2/3 pour le chemin donné, ce qui est plus propre que `remove_path()`.

### 3. `src/git/conflict.rs` — Corriger `resolve_special_file()`

Même pattern pour les fichiers spéciaux (supprimés/ajoutés) :

```rust
fn resolve_special_file(repo: &Repository, file: &MergeFile, resolution: SpecialResolution) -> Result<()> {
    let mut index = repo.index()?;
    
    match resolution {
        SpecialResolution::KeepFile(content) => {
            let full_path = repo.workdir().unwrap().join(&file.path);
            std::fs::write(&full_path, content)?;
            index.conflict_remove(Path::new(&file.path))?;
            index.add_path(Path::new(&file.path))?;
        }
        SpecialResolution::DeleteFile => {
            let full_path = repo.workdir().unwrap().join(&file.path);
            if full_path.exists() {
                std::fs::remove_file(&full_path)?;
            }
            index.conflict_remove(Path::new(&file.path))?;
            index.remove_path(Path::new(&file.path))?;
        }
    }
    
    index.write()?;
    Ok(())
}
```

### 4. `src/git/conflict.rs` — Vérification dans `finalize_merge()`

Ajouter une vérification de sécurité avant `write_tree()` :

```rust
fn finalize_merge(repo: &Repository, message: &str) -> Result<Oid> {
    let mut index = repo.index()?;
    
    // Vérification : aucun conflit ne doit subsister
    if index.has_conflicts() {
        // Lister les conflits restants pour un message d'erreur utile
        let remaining: Vec<String> = index.conflicts()?
            .filter_map(|c| c.ok())
            .filter_map(|c| {
                c.our.or(c.their).or(c.ancestor)
                    .and_then(|e| String::from_utf8(e.path).ok())
            })
            .collect();
        
        return Err(anyhow!(
            "Des conflits non résolus subsistent dans l'index : {:?}. \
             Résolvez tous les fichiers avant de finaliser.",
            remaining
        ));
    }
    
    let tree_oid = index.write_tree()?;
    // ... suite du commit
}
```

### 5. Debug : Vérifier l'état de l'index à chaque résolution

Pendant le développement, ajouter un log pour vérifier :

```rust
// Après chaque resolve_file() :
let index = repo.index()?;
eprintln!("Conflicts remaining after resolving {}: {}", file_path, index.has_conflicts());
```

## Tests

- Créer un repo de test avec un conflit de merge.
- Résoudre le conflit dans l'UI.
- Finaliser le merge : vérifier qu'aucune erreur ne se produit.
- Vérifier que le commit est créé avec les bons parents.
- Vérifier que `git log --oneline` montre le merge commit.
- Tester avec plusieurs fichiers en conflit : résoudre un par un et finaliser.
- Tester avec des fichiers spéciaux (supprimés/ajoutés).
