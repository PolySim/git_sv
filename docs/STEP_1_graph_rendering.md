# STEP 1 — Refonte du rendu du graphe (style GitKraken)

## Problème actuel

Le graphe actuel dans `graph_view.rs` est très basique :
- Lignes verticales simples `│` pour chaque colonne active
- Noeud `●` sur la colonne du commit
- **Aucune ligne diagonale** pour les forks/merges
- **Aucune connexion visible** entre un commit et ses parents sur d'autres colonnes
- Les noms de branches sont affichés entre parenthèses à côté du message, pas comme des labels visuels

## Objectif

Un graphe qui ressemble à GitKraken :
- Lignes verticales colorées par branche
- Lignes diagonales pour les forks (un commit qui crée une nouvelle branche)
- Lignes diagonales pour les merges (un commit avec 2+ parents)
- Labels de branches affichés clairement à côté du noeud correspondant
- Couleurs persistantes par branche (une branche = une couleur stable)

## Fichiers à modifier

### 1. `src/git/graph.rs` — Ajouter les données de connexion inter-lignes

Le `CommitNode` actuel ne contient que `column`. Il manque les informations
sur les lignes à dessiner **entre** deux rangées de commits.

Ajouter une struct `GraphRow` qui contient, pour chaque rangée :
```rust
pub struct GraphRow {
    /// Le commit de cette rangée.
    pub node: CommitNode,
    /// Les segments de lignes à dessiner sur cette rangée.
    pub edges: Vec<Edge>,
}

pub struct Edge {
    /// Colonne de départ (rangée du dessus).
    pub from_col: usize,
    /// Colonne d'arrivée (rangée du dessous).
    pub to_col: usize,
    /// Couleur/index de branche associée.
    pub color_index: usize,
}
```

Modifier `build_graph()` pour :
1. Suivre les colonnes actives comme aujourd'hui
2. Pour chaque commit, calculer les `Edge` :
   - Edge vertical (même colonne) pour le premier parent
   - Edge diagonal pour les parents secondaires (merge) ou les branches qui convergent/divergent
3. Nettoyer les colonnes : quand une branche est mergée, sa colonne est libérée
   et les colonnes à droite peuvent se compacter

### 2. `src/ui/graph_view.rs` — Rendu Unicode riche

Remplacer le rendu actuel par un système à deux passes :

**Passe 1 — Ligne du commit :**
```
│ │ ● abc1234  feat: ajout login (feature/login) — Simon
```
Caractères : `●` pour le noeud, `│` pour les colonnes qui passent à travers.

**Passe 2 — Ligne de connexion (entre deux commits) :**
```
│ │/ 
│ │
│/│
```
Caractères Unicode à utiliser :
- `│` ligne verticale (U+2502)
- `─` ligne horizontale (U+2500) 
- `╭` coin haut-gauche (U+256D)
- `╮` coin haut-droit (U+256E)
- `╰` coin bas-gauche (U+2570)
- `╯` coin bas-droit (U+256F)
- `/` et `\` pour les diagonales simples
- `●` noeud de commit (U+25CF)

Chaque segment est coloré selon la branche à laquelle il appartient.

### 3. `src/git/graph.rs` — Association couleur-branche stable

Actuellement la couleur dépend de `node.column % BRANCH_COLORS.len()`.
Ajouter un `HashMap<String, usize>` qui associe chaque nom de branche à un index
de couleur fixe. Quand un commit porte une ref de branche, on assigne cette couleur
à toute la "lane". Les colonnes sans ref héritent la couleur du premier parent.

### 4. Labels de branches

Dans `graph_view.rs`, afficher les labels de branches comme des badges colorés :
```
│ ● abc1234  [main] feat: initial commit — Simon
│   ● def5678  [feature/login] ajout page login — Simon
```

Le label est affiché entre crochets avec le fond coloré de la branche,
uniquement sur le commit le plus récent de cette branche (là où la ref pointe).

## Caractères de rendu — Tableau récapitulatif

| Situation            | Caractère | Description                              |
|----------------------|-----------|------------------------------------------|
| Colonne traverse     | `│`       | La branche continue verticalement        |
| Noeud commit         | `●`       | Le commit lui-même                       |
| Fork vers la droite  | `╭─`     | Une nouvelle branche part vers la droite |
| Merge depuis droite  | `╰─`     | Une branche revient vers la gauche       |
| Croisement           | `├`       | Colonne + branchement                    |

## Critère de validation

- Sur un repo avec 3+ branches et des merges, le graphe doit visuellement
  montrer les forks et merges avec des lignes diagonales colorées.
- Chaque branche garde sa couleur sur toute sa durée de vie.
- Les labels de branches sont visibles et distincts.
