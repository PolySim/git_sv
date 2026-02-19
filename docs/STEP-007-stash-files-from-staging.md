# STEP-007 — Feature : Stash de fichiers depuis la vue Staging

## Problème

Dans la vue Staging (vue 2), il n'est pas possible de stash des fichiers individuels. On ne peut actuellement stash que depuis la vue Branches (section Stashes) avec `s` qui stash tout le working directory.

## Fichiers concernés

- `src/ui/input.rs` — `map_staging_key()` (l278-333) : keybindings Staging
- `src/state.rs` — `AppAction`, `StagingFocus`
- `src/event.rs` — Handlers des actions staging
- `src/git/stash.rs` — Opérations stash

## Solution

### Étape 1 — Ajouter les actions

Dans `src/state.rs`, ajouter :

```rust
pub enum AppAction {
    // ... existants ...
    /// Stash le fichier sélectionné (vue staging).
    StashSelectedFile,
    /// Stash tous les fichiers non staged.
    StashUnstagedFiles,
}
```

### Étape 2 — Ajouter les keybindings

Dans `src/ui/input.rs`, `map_staging_key()`, ajouter la touche `S` (majuscule) pour le stash :

```rust
StagingFocus::Unstaged => match key.code {
    // ... existants ...
    KeyCode::Char('S') => Some(AppAction::StashSelectedFile),
    // ...
},
```

Pour le stash global, ajouter une touche dans les touches globales staging (par ex. `Ctrl+s` ou garder `s` comme raccourci quand le contexte est clair).

### Étape 3 — Implémenter le stash partiel

Utiliser `git2` pour stash des fichiers spécifiques. Deux approches :

#### Option A — git stash push avec pathspec (via CLI)

libgit2 ne supporte pas nativement `git stash push -- <file>`. Utiliser un subprocess :

```rust
pub fn stash_file(repo_path: &str, file_path: &str, message: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("stash").arg("push");
    if let Some(msg) = message {
        cmd.arg("-m").arg(msg);
    }
    cmd.arg("--").arg(file_path);
    cmd.current_dir(repo_path);
    
    let output = cmd.output()?;
    // ...
}
```

#### Option B — Stash tout + unstash sélectif

1. Stage les fichiers à garder
2. `git stash --keep-index`  
3. Unstage les fichiers gardés

### Recommandation

Option A est plus propre et correspond au comportement attendu. Ajouter aussi une option pour ouvrir un prompt de message de stash avant de stasher (similaire au commit message).

### Étape 4 — UI

Ajouter un indicateur dans la help bar de la vue staging :

```
s:stage  a:stage all  d:discard  S:stash  c:commit  Tab:suivant
```

## Tests

- Sélectionner un fichier non staged et faire `S` : le fichier est stashé, les autres restent
- Vérifier que le stash apparaît dans la vue Branches > Stashes
- Vérifier qu'on peut pop le stash et récupérer le fichier
- Vérifier le comportement avec plusieurs fichiers sélectionnés
