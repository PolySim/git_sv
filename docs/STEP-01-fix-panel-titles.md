# STEP-01 : Corriger les titres des panneaux Ours/Theirs

## Problème

Les panneaux affichent "Ours (HEAD)" et "Theirs (branch)" comme titres. L'utilisateur veut voir le **nom réel de la branche** (ex : `main`, `feature/xyz`) et non les termes techniques "Ours"/"Theirs".

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/state.rs` | `ConflictsState` (~427-469) | Ajouter deux champs `ours_branch_name: String` et `theirs_branch_name: String` |
| `src/git/conflict.rs` | `list_conflict_files()` (~195) | Retourner les noms de branches réels (HEAD -> nom de branche courante, MERGE_HEAD -> nom de la branche mergée) |
| `src/event.rs` | Entrées merge (~2209), pull (~1676), cherry-pick (~2055) | Passer les noms de branches lors de la construction de `ConflictsState` |
| `src/ui/conflicts_view.rs` | `render_ours_panel()` (~166), `render_theirs_panel()` (~261) | Utiliser `ours_branch_name` / `theirs_branch_name` au lieu de "Ours (HEAD)" / "Theirs (branch)" |

## Détail des modifications

### 1. `src/state.rs` — Ajouter les noms de branches

Dans `ConflictsState`, ajouter :

```rust
pub ours_branch_name: String,   // ex : "main"
pub theirs_branch_name: String, // ex : "feature/login"
```

Mettre à jour `ConflictsState::new()` pour accepter ces paramètres.

### 2. `src/git/conflict.rs` — Résoudre les noms de branches

Créer une fonction utilitaire :

```rust
/// Récupère le nom court de la branche courante (HEAD).
pub fn get_current_branch_name(repo: &Repository) -> String {
    // repo.head() -> reference -> shorthand() -> "main"
    // Fallback : "HEAD"
}

/// Récupère le nom de la branche mergée depuis MERGE_HEAD ou la description d'opération.
pub fn get_merge_branch_name(repo: &Repository) -> String {
    // Lire .git/MERGE_HEAD, trouver la branche pointant vers ce commit
    // Ou utiliser l'info passée par l'appelant (nom de branche pour merge, SHA pour cherry-pick)
    // Fallback : "MERGE_HEAD"
}
```

### 3. `src/event.rs` — Passer les noms lors de la détection de conflits

Dans `execute_merge()`, `handle_git_pull()`, `execute_cherry_pick()` :
- Le nom de la branche "ours" = branche courante (`get_current_branch_name`)
- Le nom de la branche "theirs" = branche cible du merge / SHA du cherry-pick
- Passer ces valeurs à `ConflictsState::new(files, description, ours_name, theirs_name)`

### 4. `src/ui/conflicts_view.rs` — Afficher les vrais noms

Dans `render_ours_panel()` et `render_theirs_panel()` :
- Remplacer le titre `"Ours (HEAD)"` par `state.ours_branch_name`
- Remplacer le titre `"Theirs (...)"` par `state.theirs_branch_name`
- Format suggéré : `" main "` ou `" feature/login "` (nom brut, pas de préfixe)

## Tests

- Vérifier que lors d'un merge `feature` dans `main`, le panneau gauche affiche `main` et le droit `feature`.
- Vérifier le fallback si le HEAD est détaché (devrait afficher le SHA court).
- Vérifier pour un cherry-pick (devrait afficher le SHA court du commit).
