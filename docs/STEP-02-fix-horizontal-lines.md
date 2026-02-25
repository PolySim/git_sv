# STEP-02 : Correction du bug des lignes horizontales traversantes

## Priorité : HAUTE (bug visuel)

## Problème

La fonction `find_horizontal_color()` dans `graph_view.rs` cherche la couleur d'une ligne horizontale **n'importe où dans toute la rangée**, pas seulement entre les colonnes pertinentes. Cela provoque des lignes horizontales parasites qui traversent des colonnes vides.

### Code actuel (buggé)

```rust
// src/ui/graph_view.rs, ligne ~200
fn build_connection_line(connection: &ConnectionRow) -> Line<'static> {
    // ...
    } else {
        // Colonne vide - vérifier s'il y a une ligne horizontale qui traverse.
        let has_horizontal = connection.cells.iter().any(|c| {
            c.as_ref()
                .map_or(false, |cell| cell.edge_type == EdgeType::Horizontal)
        });

        if has_horizontal {
            let horizontal_color = find_horizontal_color(col, connection);
            if let Some(color_idx) = horizontal_color {
                let color = get_branch_color(color_idx);
                spans.push(Span::styled("─", Style::default().fg(color)));
                spans.push(Span::styled("─", Style::default().fg(color)));
            } else {
                spans.push(Span::raw("  "));
            }
        } else {
            spans.push(Span::raw("  "));
        }
    }
}
```

### Pourquoi c'est un bug

Scénario : Un merge relie la colonne 0 à la colonne 3 avec des `Horizontal` aux colonnes 1 et 2. La colonne 5 est vide. Mais comme `has_horizontal` vérifie **toute la rangée**, la colonne 5 détecte "il y a un Horizontal quelque part" et dessine `──` là où il ne devrait rien y avoir.

```
Attendu :     ╰──────╮ │     │
Obtenu  :     ╰──────╮─│─────│──    ← lignes parasites
```

### Impact visuel

- Des tirets `──` apparaissent après le point d'arrivée d'un merge/fork.
- Confusion visuelle : on pense qu'il y a une connexion alors qu'il n'y en a pas.

## Fichiers impactés

| Fichier | Fonction |
|---------|----------|
| `src/ui/graph_view.rs` | `build_connection_line()`, `find_horizontal_color()` |

## Solution proposée

### Étape 1 : Supprimer la logique `has_horizontal` globale

Remplacer la vérification globale par une vérification **locale** : une colonne vide ne doit afficher un trait horizontal que si elle se trouve **entre** une source et une destination de merge/fork.

```rust
// src/ui/graph_view.rs — build_connection_line(), remplacement du bloc "colonne vide"

} else {
    // Colonne vide — vérifier si on est entre deux cellules horizontales adjacentes.
    let left_is_horizontal = col > 0
        && connection.cells.get(col - 1)
            .and_then(|c| c.as_ref())
            .map_or(false, |c| matches!(c.edge_type,
                EdgeType::Horizontal | EdgeType::MergeFromRight | EdgeType::Cross));

    let right_is_horizontal = col + 1 < connection.cells.len()
        && connection.cells.get(col + 1)
            .and_then(|c| c.as_ref())
            .map_or(false, |c| matches!(c.edge_type,
                EdgeType::Horizontal | EdgeType::ForkRight | EdgeType::ForkLeft | EdgeType::Cross));

    if left_is_horizontal && right_is_horizontal {
        // On est dans le chemin d'un merge/fork — tracer la ligne.
        let color_idx = find_horizontal_color_bounded(col, connection);
        if let Some(idx) = color_idx {
            let color = get_branch_color(idx);
            spans.push(Span::styled("─", Style::default().fg(color)));
            spans.push(Span::styled("─", Style::default().fg(color)));
        } else {
            spans.push(Span::raw("  "));
        }
    } else {
        spans.push(Span::raw("  "));
    }
}
```

### Étape 2 : Réécrire `find_horizontal_color()` en version bornée

```rust
/// Trouve la couleur d'une ligne horizontale adjacente (recherche bornée).
/// Ne cherche que dans les cellules immédiatement voisines.
fn find_horizontal_color_bounded(
    col: usize,
    connection: &ConnectionRow,
) -> Option<usize> {
    // Chercher vers la gauche (la cellule la plus proche).
    for c in (0..col).rev() {
        match &connection.cells[c] {
            Some(cell) if cell.edge_type == EdgeType::Horizontal => {
                return Some(cell.color_index);
            }
            Some(cell) if matches!(cell.edge_type,
                EdgeType::MergeFromRight | EdgeType::MergeFromLeft) => {
                return Some(cell.color_index);
            }
            Some(_) => break, // Autre type de cellule = on arrête
            None => continue, // Colonne vide = on continue
        }
    }

    // Chercher vers la droite.
    for c in (col + 1)..connection.cells.len() {
        match &connection.cells[c] {
            Some(cell) if cell.edge_type == EdgeType::Horizontal => {
                return Some(cell.color_index);
            }
            Some(cell) if matches!(cell.edge_type,
                EdgeType::ForkRight | EdgeType::ForkLeft) => {
                return Some(cell.color_index);
            }
            Some(_) => break,
            None => continue,
        }
    }

    None
}
```

### Étape 3 : Corriger aussi l'espacement horizontal entre cellules

Le code actuel dans `build_connection_line()` ajoute un `─` d'espacement entre deux cellules si la cellule suivante est `Horizontal`. Ce calcul est correct mais doit aussi être vérifié :

```rust
// Lignes ~185-195 — vérifier que la condition est correcte
let needs_horizontal_right = col + 1 < num_cols
    && connection.cells[col + 1]
        .as_ref()
        .map_or(false, |c| c.edge_type == EdgeType::Horizontal);
```

Ce check est OK car il ne regarde que la cellule **immédiatement à droite**.

## Tests à ajouter

```rust
#[test]
fn test_no_horizontal_leak_past_fork() {
    // Créer une ConnectionRow avec merge col0→col2, colonne 3 vide
    let connection = ConnectionRow {
        cells: vec![
            Some(GraphCell { edge_type: EdgeType::MergeFromRight, color_index: 0 }),
            Some(GraphCell { edge_type: EdgeType::Horizontal, color_index: 0 }),
            Some(GraphCell { edge_type: EdgeType::ForkRight, color_index: 0 }),
            None, // ← Cette colonne NE doit PAS avoir de "──"
            Some(GraphCell { edge_type: EdgeType::Vertical, color_index: 1 }),
        ],
    };
    let line = build_connection_line(&connection);
    // Vérifier que les spans après col 2 ne contiennent pas "─"
    // (sauf l'espacement normal entre col2 et col3)
}
```

## Critère de validation

- Les lignes horizontales ne dépassent jamais les points d'arrivée (╮/╭).
- Les colonnes vides après un fork/merge restent vides (espaces).
- Visuellement, le graphe est propre sans "traînées" horizontales.
- `cargo test` passe.
- `cargo clippy` ne lève pas de warnings.
