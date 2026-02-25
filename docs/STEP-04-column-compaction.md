# STEP-04 : Compaction des colonnes du graphe

## Priorité : MOYENNE (UX)

## Problème

Le graphe ne compacte jamais ses colonnes. Quand une branche est mergée et sa colonne libérée (`expected_oid = None`), la colonne reste dans `active_columns` et peut être réutilisée **mais** les colonnes inutilisées à droite ne sont jamais supprimées. Résultat : le graphe s'élargit indéfiniment vers la droite au fil de l'historique.

### Code actuel

```rust
// src/git/graph.rs — assign_new_column()

fn assign_new_column(active_columns: &mut Vec<ColumnState>, oid: Oid) -> usize {
    // Chercher la première colonne libre (réutilisation)
    for (i, state) in active_columns.iter_mut().enumerate() {
        if state.expected_oid.is_none() {
            state.expected_oid = Some(oid);
            return i;
        }
    }
    // Aucune libre → créer une nouvelle colonne
    active_columns.push(ColumnState { ... });
    active_columns.len() - 1
}
```

La réutilisation fonctionne pour les trous internes, mais les colonnes terminales vides ne sont jamais supprimées. Exemple :

```
Commit 1:  ●  │  │  │  │     ← 5 colonnes actives
Commit 2:  ●  │  │  │        ← 4 actives mais active_columns.len() reste 5
Commit 3:  ●  │  │           ← 3 actives mais active_columns.len() reste 5
```

L'espacement est dessiné pour les 5 colonnes, gaspillant de l'espace horizontal.

### Impact visuel

- Le graphe consomme inutilement de l'espace horizontal.
- Moins de place pour le message du commit, l'auteur, la date.
- Sur des repos avec beaucoup de branches mergées, la partie graphe peut occuper 30+ colonnes pour seulement 2-3 branches actives.

## Fichiers impactés

| Fichier | Fonction |
|---------|----------|
| `src/git/graph.rs` | `build_graph()` — boucle principale |
| `src/ui/graph_view.rs` | `build_commit_line()`, `build_connection_line()` — calcul de `num_cols` |

## Solution proposée

### Étape 1 : Tronquer `active_columns` après chaque commit

Après avoir traité un commit et ses parents, supprimer les colonnes vides à la fin de `active_columns` :

```rust
// src/git/graph.rs — dans build_graph(), après assign_parent_columns()

// Compacter : supprimer les colonnes vides en fin de vecteur.
while active_columns.last().map_or(false, |s| s.expected_oid.is_none()) {
    active_columns.pop();
}
```

### Étape 2 : Adapter `build_commit_cells()` et `build_connection_row()`

Ces fonctions utilisent `active_columns.len()` pour déterminer le nombre de colonnes à dessiner. Après la compaction, cette valeur sera correcte automatiquement. Aucun changement nécessaire dans ces fonctions.

### Étape 3 : Adapter `num_cols` dans `graph_view.rs`

Le calcul de `num_cols` utilise déjà :
```rust
let num_cols = row.cells.len().max(node.column + 1);
```

Après compaction, `row.cells.len()` sera plus petit. Le `.max(node.column + 1)` garantit que la colonne du commit est toujours incluse. Pas de changement nécessaire.

### Considérations

- La compaction ne doit se faire que par la droite (les colonnes internes vides doivent rester pour maintenir l'alignement des branches actives).
- Il faut compacter **avant** de calculer les cellules et la connexion pour ce commit, ou **après** pour que la connexion reflète l'état compacté. L'ordre optimal est :
  1. Assigner le commit à sa colonne
  2. Construire les cellules du commit (avec l'état complet)
  3. Libérer la colonne du commit
  4. Assigner les parents
  5. **Compacter**
  6. Construire la connexion (avec l'état compacté)

```rust
// src/git/graph.rs — dans build_graph()

// ... (existant: build_commit_cells, libérer colonne, assign_parent_columns)

// Compacter les colonnes terminales vides.
while active_columns.last().map_or(false, |s| s.expected_oid.is_none()) {
    active_columns.pop();
}

// Générer la ligne de connexion APRÈS compaction.
let connection = if commit_idx + 1 < commits.len() {
    Some(build_connection_row(&active_columns, &parent_assignments, column))
} else {
    None
};
```

### Attention : impact sur les `parent_assignments`

Les `parent_assignments` référencent des indices de colonnes qui pourraient avoir été supprimées par la compaction. Il faut s'assurer que seules les colonnes terminales vides (sans parent assigné) sont supprimées. Comme `assign_parent_columns` assigne les parents avant la compaction, les colonnes avec des parents ne seront jamais vides et ne seront pas supprimées.

## Tests à ajouter

```rust
#[test]
fn test_column_compaction() {
    // Créer un scénario : branche créée puis mergée
    // Vérifier que active_columns.len() diminue après le merge
    let (_temp, repo) = create_test_repo();
    let oid_a = commit_file(&repo, "file.txt", "A", "Initial");
    
    // Créer une branche, commiter, revenir, merger
    create_branch_and_commit(&repo, "feature", "feat.txt", "feat", "Feature commit");
    checkout(&repo, "main");
    let oid_c = commit_file(&repo, "file.txt", "C", "Main continues");
    merge(&repo, "feature");
    let oid_d = commit_file(&repo, "file.txt", "D", "After merge");
    
    let commits = collect_commits(&repo);
    let graph = build_graph(&repo, &commits).unwrap();
    
    // Après le merge, le dernier commit devrait être sur la colonne 0
    // et le nombre de colonnes dans cells devrait revenir à 1
    let last = graph.last().unwrap();
    assert_eq!(last.cells.len(), 1, "Les colonnes devraient être compactées");
}
```

## Critère de validation

- Le graphe ne s'élargit pas indéfiniment.
- Après un merge, les colonnes inutilisées sont récupérées.
- L'alignement visuel reste correct (pas de décalage entre les lignes).
- Les tests existants continuent de passer.
- `cargo test` et `cargo clippy` OK.
