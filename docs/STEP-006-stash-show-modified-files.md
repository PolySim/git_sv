# STEP-006 — Feature : Voir les fichiers modifiés dans les stash

## Problème

Quand on parcourt les stash dans la vue Branches (section Stashes), on ne voit que le message du stash mais pas les fichiers modifiés qu'il contient. Il devrait y avoir un panneau de détail montrant la liste des fichiers.

## Fichiers concernés

- `src/state.rs` — `BranchesViewState` (l275), `StashEntry` dans les stashes
- `src/git/stash.rs` — Structure `StashEntry` et opérations stash
- `src/ui/branches_view.rs` — Rendu de la vue Branches (panneau détail droit)
- `src/event.rs` — Handler de navigation dans les stashes

## Analyse

Actuellement, `StashEntry` contient probablement uniquement le message et l'index du stash. Le panneau de détail (droite) dans la vue Branches n'affiche pas d'informations spécifiques pour les stashes.

## Solution

### Étape 1 — Enrichir `StashEntry` avec les fichiers modifiés

Ajouter une méthode pour lister les fichiers d'un stash via `git2` :

```rust
// Dans src/git/stash.rs
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    pub oid: git2::Oid,
}

/// Récupère la liste des fichiers modifiés dans un stash.
pub fn stash_files(repo: &Repository, stash_oid: git2::Oid) -> Result<Vec<DiffFile>> {
    let stash_commit = repo.find_commit(stash_oid)?;
    let stash_tree = stash_commit.tree()?;
    
    // Le parent du stash est le commit sur lequel il a été créé
    let parent = stash_commit.parent(0)?;
    let parent_tree = parent.tree()?;
    
    let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&stash_tree), None)?;
    
    // Extraire les fichiers du diff
    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                files.push(DiffFile {
                    path: path.to_string_lossy().to_string(),
                    status: delta.status(),
                });
            }
            true
        },
        None, None, None,
    )?;
    
    Ok(files)
}
```

### Étape 2 — Afficher les fichiers dans le panneau détail

Quand la section active est `Stashes` et qu'un stash est sélectionné, le panneau détail droit affiche :
- Le message du stash
- La branche sur laquelle il a été créé
- La liste des fichiers modifiés avec leur statut (M/A/D)

```rust
// Dans src/ui/branches_view.rs, section rendu du détail
BranchesSection::Stashes => {
    if let Some(stash) = state.stashes.get(state.stash_selected) {
        // Afficher message + fichiers
        lines.push(format!("Stash: {}", stash.message));
        lines.push(String::new());
        lines.push("Fichiers modifiés:".to_string());
        for file in &stash.files {
            lines.push(format!("  {} {}", file.status_char(), file.path));
        }
    }
}
```

### Étape 3 — Charger les fichiers à la sélection

Quand l'utilisateur navigue dans les stashes (MoveUp/MoveDown en section Stashes), charger les fichiers du stash sélectionné et les stocker dans l'état.

## Tests

- Sélectionner un stash et vérifier que les fichiers modifiés sont affichés
- Naviguer entre les stashes et vérifier que la liste se met à jour
- Vérifier avec un stash contenant des fichiers ajoutés, modifiés, supprimés
- Vérifier le cas d'un stash vide (stash d'index uniquement)
