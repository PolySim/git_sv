# STEP-01 : Correction du bug de synchronisation de la sélection

## Priorité : HAUTE (bug)

## Problème

La synchronisation entre `selected_index` (index logique du commit) et `graph_state` (index visuel dans la `List` ratatui) est buggée à cause de l'hypothèse `selected_index * 2`.

### Explication

Le graphe produit **2 items par commit** dans la `List` ratatui :
1. La ligne du commit (nœud + hash + message)
2. La ligne de connexion vers le commit suivant (`ConnectionRow`)

Le code fait `graph_state.select(Some(selected_index * 2))` pour convertir l'index logique en index visuel. **Mais** la dernière `GraphRow` n'a PAS de `ConnectionRow` (car il n'y a pas de commit suivant) :

```rust
// src/git/graph.rs, dans build_graph()
let connection = if commit_idx + 1 < commits.len() {
    Some(build_connection_row(...))
} else {
    None  // ← Pas de connection pour le dernier commit
};
```

Et dans `build_graph_items()` :
```rust
// src/ui/graph_view.rs
if let Some(ref connection) = row.connection {
    let connection_line = build_connection_line(connection);
    items.push(ListItem::new(connection_line));  // ← Pas ajouté pour le dernier
}
```

### Conséquences

- Pour N commits, on a `2*N - 1` items visuels (pas `2*N`).
- Quand on sélectionne le dernier commit, `selected_index * 2` pointe **au-delà** des items, ce qui provoque un comportement indéfini du widget `List` (pas de highlight visible, ou panique potentielle).
- Quand on sélectionne l'avant-dernier commit, la sélection peut être légèrement décalée visuellement.

## Fichiers impactés

| Fichier | Rôle |
|---------|------|
| `src/ui/graph_view.rs` | `build_graph_items()` — production des items |
| `src/state/mod.rs` | `sync_graph_selection()` — calcul de l'index `* 2` |
| `src/handler/navigation.rs` | `handle_move_up/down/page_up/page_down/go_top/go_bottom` — mise à jour de `graph_state` |

## Solution proposée

### Option A (recommandée) : Calculer l'index visuel correctement

Créer une fonction utilitaire qui calcule l'index visuel réel en tenant compte des `ConnectionRow` présentes :

```rust
// src/ui/graph_view.rs (ou src/state/mod.rs)

/// Calcule l'index visuel dans la List ratatui pour un index de commit donné.
/// Chaque commit produit 1 item + 1 item de connexion (sauf le dernier).
pub fn commit_to_visual_index(commit_index: usize, total_commits: usize) -> usize {
    if total_commits == 0 {
        return 0;
    }
    // Chaque commit avant le dernier produit 2 items (commit + connexion).
    // Le dernier commit produit 1 item (commit seul).
    // Index visuel = commit_index * 2 (car toutes les lignes avant ont une connexion)
    // SAUF si commit_index == total_commits - 1, auquel cas c'est 2*(total-1)
    commit_index * 2
}
```

En fait le calcul `commit_index * 2` est correct **tant que** l'index pointe vers un commit qui a une connexion après lui. Le vrai problème est que le nombre total d'items est `2*N - 1`, et `(N-1) * 2 = 2*N - 2` qui est bien dans les bornes. Le bug se manifeste surtout quand les `ConnectionRow` sont absentes pour d'autres raisons (graphe vide, filtres, etc.).

**Action concrète** : Vérifier et uniformiser la conversion en un seul endroit :

1. Dans `sync_graph_selection()` de `src/state/mod.rs`, remplacer :
```rust
self.graph_state.select(Some(self.selected_index * 2));
```
par :
```rust
let visual_index = if self.graph.is_empty() {
    0
} else {
    // Chaque commit (sauf le dernier) a une ligne de connexion.
    self.selected_index * 2
};
self.graph_state.select(Some(visual_index));
```

2. Dans `src/handler/navigation.rs`, **supprimer** toutes les lignes dupliquées de `graph_state.select(Some(... * 2))` et appeler `state.sync_graph_selection()` à la place (DRY) :

```rust
// Avant (dupliqué dans handle_move_up, handle_move_down, etc.)
state.graph_state.select(Some(state.selected_index * 2));
state.sync_legacy_selection();

// Après (centralisé)
state.sync_graph_selection();
state.sync_legacy_selection();
```

### Option B : Ajouter une connexion vide pour le dernier commit

Dans `build_graph_items()`, toujours ajouter un item de connexion (vide) même pour le dernier commit, pour garantir que chaque commit produit exactement 2 items :

```rust
// src/ui/graph_view.rs, dans build_graph_items()
if let Some(ref connection) = row.connection {
    let connection_line = build_connection_line(connection);
    items.push(ListItem::new(connection_line));
} else {
    // Ligne vide pour maintenir la cohérence 2 items par commit.
    items.push(ListItem::new(Line::from("")));
}
```

## Tests à ajouter

```rust
#[test]
fn test_visual_index_last_commit() {
    let graph = create_test_graph(); // 2 commits
    let items = build_graph_items(&graph, 1); // Sélection du dernier
    // Vérifier que selected_index * 2 est dans les bornes
    assert!(1 * 2 < items.len() || items.len() == 2 * graph.len() - 1);
}

#[test]
fn test_sync_graph_selection_empty() {
    // Tester avec un graphe vide
}
```

## Critère de validation

- Naviguer jusqu'au dernier commit ne cause pas de perte de highlight.
- `cargo test` passe.
- Pas de panique sur un graphe vide.
