# STEP 3 — Vue des fichiers modifiés par le commit sélectionné

## Problème actuel

- Le panneau `status_view.rs` affiche le status du working directory (fichiers
  modifiés/staged/untracked) mais **pas** les fichiers modifiés par un commit donné.
- Quand on navigue dans le graphe et qu'on sélectionne un commit, on ne voit que
  ses métadonnées (hash, auteur, date, message) dans `detail_view.rs`.
- Aucun moyen de savoir quels fichiers ont été touchés par un commit.

## Objectif

Quand un commit est sélectionné dans le graphe, le panneau bas-gauche affiche
la liste des fichiers modifiés par ce commit avec :
- Le type de modification (Ajouté, Modifié, Supprimé, Renommé)
- Le nombre de lignes ajoutées/supprimées (+42 / -12)
- Couleurs : vert pour ajouts, rouge pour suppressions

Le panneau bascule entre deux modes :
- **Mode commit** (par défaut quand on navigue le graphe) : fichiers du commit sélectionné
- **Mode working dir** (avec `Tab`) : status du working directory actuel

## Fichiers à créer/modifier

### 1. Ajouter `src/git/diff.rs` — Calcul du diff d'un commit

```rust
pub struct DiffFile {
    pub path: String,
    pub status: DiffStatus,       // Added, Modified, Deleted, Renamed
    pub old_path: Option<String>,  // En cas de rename
    pub additions: usize,
    pub deletions: usize,
}

pub enum DiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}
```

Fonction principale :
```rust
pub fn commit_diff(repo: &Repository, oid: Oid) -> Result<Vec<DiffFile>>
```

Logique :
1. Trouver le commit via `repo.find_commit(oid)`
2. Obtenir l'arbre du commit et l'arbre de son premier parent
   (si pas de parent = premier commit, diff avec arbre vide)
3. Utiliser `repo.diff_tree_to_tree(parent_tree, commit_tree, None)`
4. Itérer sur les deltas pour remplir `DiffFile`
5. Utiliser `diff.stats()` ou itérer les hunks pour les compteurs +/-

### 2. Modifier `src/git/mod.rs`

```rust
pub mod diff;
pub use diff::DiffFile;
```

### 3. Modifier `src/git/repo.rs` — Ajouter méthode `commit_diff()`

```rust
pub fn commit_diff(&self, oid: git2::Oid) -> Result<Vec<DiffFile>> {
    super::diff::commit_diff(&self.repo, oid)
}
```

### 4. Modifier `src/app.rs` — État du diff

Ajouter dans `App` :
```rust
pub commit_files: Vec<DiffFile>,       // Fichiers du commit sélectionné
pub bottom_left_mode: BottomLeftMode,  // CommitFiles ou WorkingDir

pub enum BottomLeftMode {
    CommitFiles,
    WorkingDir,
}
```

Quand `selected_index` change (MoveUp/MoveDown), recalculer `commit_files` :
```rust
if let Some(node) = self.selected_commit() {
    self.commit_files = self.repo.commit_diff(node.oid).unwrap_or_default();
}
```

`Tab` bascule entre `BottomLeftMode::CommitFiles` et `BottomLeftMode::WorkingDir`.

### 5. Refactorer `src/ui/status_view.rs` — Vue contextuelle

Renommer en `files_view.rs` (ou garder le nom mais adapter).

Le rendu dépend de `app.bottom_left_mode` :
- `CommitFiles` : affiche `app.commit_files` avec format :
  ```
  M  +42 -12  src/main.rs
  A  +85  -0  src/git/diff.rs
  D   -0 -34  src/old_file.rs
  ```
- `WorkingDir` : affiche `app.status_entries` (comportement actuel)

Le titre du bloc change aussi :
- `" Fichiers — abc1234 "` en mode commit
- `" Status (3 fichiers) "` en mode working dir

### 6. Modifier `src/ui/mod.rs`

Mettre à jour le render pour passer le bon mode.

## Critère de validation

- Quand on navigue dans le graphe (j/k), le panneau bas-gauche affiche les fichiers
  modifiés par le commit sous le curseur.
- On voit les stats +/- en vert/rouge.
- `Tab` bascule entre la vue commit et le status du working directory.
