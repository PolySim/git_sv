# STEP 1 — Refonte du graphe git (style GitKraken)

## Objectif

Transformer le rendu du graphe git pour qu'il ressemble à GitKraken : lignes continues
et fluides, merges clairement visibles avec des courbes, couleurs stables par branche,
et une lecture naturelle du flux des branches.

---

## Problèmes actuels

1. **Lignes discontinues** : Le graphe actuel utilise des caractères simples (`│`, `╭`, `╰`)
   avec des espaces entre les colonnes, ce qui donne un rendu haché.
2. **Merges peu lisibles** : Quand un merge arrive, on ne voit pas clairement d'où vient
   la branche qui fusionne. Les diagonales sont basiques.
3. **Pas de lignes entre les commits** : Les edge lines entre deux rangées de commits
   sont simplistes (juste `│` vertical ou `╭`/`╰`).
4. **Couleurs instables** : L'attribution des couleurs utilise la colonne comme fallback,
   ce qui peut changer d'un refresh à l'autre.

---

## Plan d'implémentation

### 1.1 — Refonte de la structure de données du graphe (`src/git/graph.rs`)

**Fichier** : `src/git/graph.rs`

#### Nouveaux types à introduire

```rust
/// Type de segment visuel dans le graphe.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    /// Ligne verticale continue (│).
    Vertical,
    /// Courbe vers la droite depuis le commit (╭─).
    ForkRight,
    /// Courbe vers la gauche depuis le commit (─╮).
    ForkLeft,
    /// Merge : courbe entrante depuis la droite (╰─).
    MergeFromRight,
    /// Merge : courbe entrante depuis la gauche (─╯).
    MergeFromLeft,
    /// Ligne horizontale de passage (─).
    Horizontal,
    /// Croisement de lignes (┼).
    Cross,
}

/// Cellule du graphe : représente ce qui est dessiné dans une colonne donnée.
#[derive(Debug, Clone)]
pub struct GraphCell {
    /// Type de segment à dessiner.
    pub edge_type: EdgeType,
    /// Index de couleur de la branche.
    pub color_index: usize,
}

/// Rangée intermédiaire entre deux commits (pour les connexions).
#[derive(Debug, Clone)]
pub struct ConnectionRow {
    /// Cellules de connexion pour chaque colonne.
    pub cells: Vec<Option<GraphCell>>,
}
```

#### Modifications de `GraphRow`

```rust
pub struct GraphRow {
    pub node: CommitNode,
    /// Cellules du graphe sur la ligne du commit (colonnes actives).
    pub cells: Vec<Option<GraphCell>>,
    /// Ligne de connexion vers la rangée suivante.
    pub connection: Option<ConnectionRow>,
}
```

#### Amélioration de l'algorithme `build_graph()`

L'algorithme actuel assigne déjà les colonnes correctement. Les améliorations :

1. **Traquer la couleur par colonne** (pas par ref) : Quand une colonne est occupée par
   un flux de commits, elle garde sa couleur même si les commits n'ont pas de refs.
   Ajouter un `Vec<usize>` qui mappe chaque colonne active à un `color_index`.

2. **Générer les `GraphCell` pour chaque commit** : Pour la ligne du commit, chaque
   colonne active reçoit une cellule `Vertical`, et la colonne du commit reçoit le nœud.

3. **Générer les `ConnectionRow`** : Entre deux rangées de commits, calculer les
   connexions :
   - Si un edge va de `col A` à `col A` → `Vertical`
   - Si un edge va de `col A` à `col B` (B > A) → `ForkRight` en A, `Horizontal` entre, `MergeFromLeft` en B
   - Si un edge va de `col A` à `col B` (B < A) → `ForkLeft` en A, `Horizontal` entre, `MergeFromRight` en B

4. **Compacter les colonnes** : Quand une colonne se libère (branche terminée), réutiliser
   l'espace pour les nouvelles branches. Déjà fait dans `assign_new_column()`.

5. **Gérer les merges multi-parents** : Pour un commit avec N parents, le premier parent
   reste dans la même colonne. Les parents supplémentaires créent des courbes distinctes
   avec la couleur de la branche source.

---

### 1.2 — Refonte du rendu graphique (`src/ui/graph_view.rs`)

**Fichier** : `src/ui/graph_view.rs`

#### Caractères Unicode à utiliser

```
Nœuds :
  ● — commit normal
  ◉ — commit sélectionné (ou HEAD)
  ○ — merge commit

Lignes verticales :
  │ — ligne continue verticale

Courbes (style arrondi pour un look GitKraken) :
  ╭ — coin haut-gauche (fork vers la droite)
  ╮ — coin haut-droit (fork vers la gauche)
  ╰ — coin bas-gauche (merge depuis la droite)
  ╯ — coin bas-droit (merge depuis la gauche)

Lignes horizontales :
  ─ — connexion horizontale

Intersections :
  ├ — jonction à droite
  ┤ — jonction à gauche
  ┼ — croisement
```

#### Refonte de `build_commit_line()`

Parcourir les `GraphCell` de la rangée au lieu de calculer les edges à la volée.
Chaque cellule produit le bon caractère Unicode avec la bonne couleur.

```rust
fn build_commit_line(row: &GraphRow, is_selected: bool) -> Line<'static> {
    let mut spans = Vec::new();

    for (col, cell) in row.cells.iter().enumerate() {
        if col == row.node.column {
            // Dessiner le nœud.
            let symbol = if row.node.parents.len() > 1 { "○" } else { "●" };
            spans.push(Span::styled(symbol, ...));
        } else if let Some(cell) = cell {
            // Dessiner la ligne de passage.
            let ch = match cell.edge_type {
                EdgeType::Vertical => "│",
                _ => " ",
            };
            spans.push(Span::styled(ch, color));
        } else {
            spans.push(Span::raw(" "));
        }
    }

    // Puis le hash, les refs, le message, l'auteur...
}
```

#### Refonte de `build_edge_line()` → `build_connection_line()`

Utiliser la `ConnectionRow` pour dessiner les connexions avec les bons caractères
de courbes. C'est ici que la magie GitKraken opère :

```
Exemple de rendu pour un merge :
  ● ── commit A (branche main)
  │ ╲
  │  ● ── commit B (branche feature)
  │  │
  │  ● ── commit C
  │ ╱
  ● ── merge commit
```

En termes de cellules :

```
Col 0  Col 1
  ●              ← commit (main)
  │    ╭─        ← connection : vertical en 0, fork en 1
  │    ●         ← commit (feature)
  │    │         ← connection : vertical en 0, vertical en 1
  │    ●         ← commit
  ╰─   │        ← connection : merge en 0 depuis 1
  ●              ← merge commit
```

#### Espacement des colonnes

Utiliser un espacement de **2 caractères** entre chaque colonne pour que les lignes
horizontales puissent passer (`─`) et que les courbes soient lisibles.

---

### 1.3 — Couleurs stables par branche

**Fichier** : `src/git/graph.rs`

#### Stratégie

1. Maintenir un `HashMap<Oid, usize>` qui associe le premier commit d'une branche à un
   color_index.
2. Quand un commit a des refs (ex: `main`, `feature/foo`), utiliser le nom de la ref
   pour l'index de couleur (déjà fait partiellement).
3. Quand un commit n'a pas de ref, propager la couleur de son enfant (le commit qui
   pointe vers lui via parent). Cela se fait en stockant la couleur par colonne active.
4. Ajouter un `Vec<usize>` `column_colors` dans l'algorithme, mis à jour à chaque
   assignation de colonne.

---

### 1.4 — Gestion des colonnes fermées et compactage

Quand une branche se termine (merge dans une autre), sa colonne doit être libérée et
les colonnes à droite doivent éventuellement se décaler pour ne pas laisser de trous
visuels. 

**Option A** (simple) : Laisser le trou, il sera comblé naturellement par `assign_new_column()`.
**Option B** (GitKraken-like) : Ajouter une étape de compactage où les colonnes se
rapprochent progressivement. Cela nécessite des edges diagonaux supplémentaires.

→ **Recommandation** : Commencer par Option A, puis implémenter Option B dans un second
temps si le rendu n'est pas satisfaisant.

---

## Fichiers impactés

| Fichier | Modifications |
|---------|--------------|
| `src/git/graph.rs` | Nouveaux types (`EdgeType`, `GraphCell`, `ConnectionRow`), refonte de `build_graph()` |
| `src/ui/graph_view.rs` | Refonte complète du rendu avec les nouveaux types |
| `src/app.rs` | Adaptation si la structure de `GraphRow` change (minime) |
| `src/ui/mod.rs` | Aucun changement (le render passe toujours `&[GraphRow]`) |

---

## Critères de validation

- [ ] Les lignes de branches sont continues (pas d'interruption entre deux commits)
- [ ] Les merges sont clairement visibles avec des courbes
- [ ] Les forks (création de branche) sont visibles avec des courbes
- [ ] Les couleurs sont stables : une branche garde sa couleur du début à la fin
- [ ] Le graphe reste lisible avec 5+ branches parallèles
- [ ] Les performances restent bonnes (200 commits en < 50ms)
- [ ] Le rendu est correct sur des terminaux de différentes largeurs
- [ ] `cargo clippy` ne génère aucun warning

---

## Inspirations visuelles (GitKraken)

```
  ●──── main: "fix: correction du bug #42"
  │
  │  ●── feature/auth: "feat: ajout login"
  │  │
  │  ●── "feat: ajout register"
  │  │
  ●──╯── "Merge branch 'feature/auth'"
  │
  │  ●── feature/ui: "refactor: nouveau layout"
  │  │
  ●──╯── "Merge branch 'feature/ui'"
  │
  ●──── "chore: bump dependencies"
```

Les lignes sont lisses, continues, et on voit immédiatement le flux des branches.
