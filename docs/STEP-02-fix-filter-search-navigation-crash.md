# STEP-02 — Corriger les crashs de navigation lors du filtre/recherche

## Problème

Lors de la recherche par filtre, la navigation est complètement buguée et fait crasher l'application. Plusieurs causes racines ont été identifiées :

### Cause 1 : `graph_state` (ListState) jamais mis à jour après `refresh()`

Dans `handler/mod.rs`, la fonction `refresh()` reconstruit le graphe et clampe `selected_index`, mais **ne met jamais à jour `graph_state`** (le `ListState` de ratatui). Résultat : `graph_state` pointe vers un index périmé qui dépasse potentiellement le nombre d'items de la liste → comportement erratique ou panic au rendu.

### Cause 2 : Incohérence `index` vs `index * 2` dans `graph_state`

Le graphe ratatui contient **2 items par commit** (ligne de commit + ligne de connexion). Les handlers de navigation utilisent `graph_state.select(Some(index * 2))`, mais :
- `refresh()` ne met pas à jour `graph_state` du tout
- `sync_graph_selection()` dans `state/mod.rs` utilise `selected_index` sans le `* 2`

### Cause 3 : Résultats de recherche périmés après application d'un filtre

Quand un filtre est appliqué (`FilterAction::Apply`), le graphe est reconstruit avec moins de commits. Mais `search_state.results` (qui contient des index dans l'ancien graphe) **n'est pas invalidé**. Naviguer vers un résultat de recherche après filtrage pointe vers le mauvais commit ou vers un index hors limites.

### Cause 4 : `file_selected_index` pas clampé après changement de commit

Quand la navigation saute à un autre commit (via recherche ou filtre), `file_selected_index` garde sa valeur précédente. Si le nouveau commit a moins de fichiers, l'accès `commit_files[file_selected_index]` dans `handler/git.rs:181` **panique** (index direct sans `.get()`).

### Cause 5 : Scroll souris traverse le popup de filtre

`map_mouse()` dans `ui/input.rs` ne vérifie pas `state.filter_popup.is_open`. Le scroll souris dispatch `MoveUp`/`MoveDown` même quand le popup est visible, ce qui déplace silencieusement la sélection derrière le popup.

## Fichiers concernés

| Fichier | Problème |
|---------|----------|
| `src/handler/mod.rs` | `refresh()` ne synchronise pas `graph_state` |
| `src/state/mod.rs` | `sync_graph_selection()` utilise `index` au lieu de `index * 2` |
| `src/handler/filter.rs` | `handle_apply/clear` ne réinitialisent pas `search_state` |
| `src/handler/search.rs` | Résultats périmés après filtrage ; `handle_execute` navigue au 2e résultat au lieu du 1er |
| `src/handler/git.rs:181` | Accès non protégé `commit_files[file_selected_index]` |
| `src/ui/input.rs` | `map_mouse()` ne bloque pas les events quand le popup filtre est ouvert |

## Corrections à apporter

### 1. Synchroniser `graph_state` dans `refresh()`

Dans `handler/mod.rs`, après le clamping de `selected_index` :

```rust
// Après le clamping
self.state.graph_state.select(Some(self.state.selected_index * 2));
```

### 2. Corriger `sync_graph_selection()`

Dans `state/mod.rs` :

```rust
pub fn sync_graph_selection(&mut self) {
    self.selected_index = self.graph_view.rows.selected_index();
    self.graph_state.select(Some(self.selected_index * 2)); // ← * 2 ajouté
}
```

### 3. Invalider la recherche quand un filtre est appliqué/effacé

Dans `handler/filter.rs`, `handle_apply()` et `handle_clear()` :

```rust
// Réinitialiser l'état de recherche
state.search_state.results.clear();
state.search_state.current_result = 0;
state.search_state.is_active = false;
state.search_state.query.clear();
```

### 4. Clamper `file_selected_index` dans `refresh()`

Dans `handler/mod.rs`, après la reconstruction du graphe :

```rust
if self.state.file_selected_index >= self.state.commit_files.len() {
    self.state.file_selected_index = self.state.commit_files.len().saturating_sub(1);
}
```

### 5. Protéger l'accès à `commit_files` dans `handler/git.rs`

Remplacer :
```rust
let selected_file = &state.commit_files[state.file_selected_index];
```
Par :
```rust
let selected_file = match state.commit_files.get(state.file_selected_index) {
    Some(f) => f,
    None => return Ok(()),
};
```

### 6. Bloquer les events souris pendant le popup filtre

Dans `ui/input.rs`, `map_mouse()` :

```rust
if state.filter_popup.is_open {
    return None;
}
```

### 7. Corriger `handle_execute` dans search.rs

Le `handle_execute` appelle `handle_next_result` qui incrémente `current_result` de 0 à 1. Pour naviguer au premier résultat, initialiser à `-1` (ou `results.len() - 1`) avant l'appel, ou naviguer directement sans passer par `handle_next_result`.

## Vérification

- `cargo build` compile
- `cargo test` passe
- `cargo clippy` sans warning
- Tester : ouvrir un repo avec beaucoup de commits, appliquer un filtre qui réduit fortement la liste, naviguer haut/bas → pas de crash
- Tester : effectuer une recherche, puis appliquer un filtre → les résultats de recherche sont réinitialisés
- Tester : ouvrir le popup filtre et scroller à la souris → pas de mouvement derrière le popup
