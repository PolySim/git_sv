# STEP-05 : Amélioration de la lisibilité du graphe

## Priorité : MOYENNE (UX)

## Problèmes identifiés

### 5.1 — L'espacement `COL_SPACING = 2` est trop serré

Avec un espacement de 2 caractères entre colonnes, les lignes de connexion (╮, ╭, ─) sont collées et difficiles à distinguer visuellement, surtout avec 4+ branches.

### 5.2 — Le message de commit n'est pas tronqué

Si le message est long, la ligne dépasse la largeur du terminal et est tronquée par ratatui sans ellipsis. L'auteur et la date (à la fin) sont les premières informations perdues.

### 5.3 — Pas de séparation visuelle entre la partie graphe et la partie texte

La transition entre les colonnes du graphe et le hash est un simple espace ` `. Quand le graphe a beaucoup de colonnes vides à droite, le hash semble "flottant".

### 5.4 — La date relative n'est pas assez distincte de l'auteur

`— Alice (il y a 2h)` utilise le même style `text_secondary` pour l'auteur et la date. Les deux se confondent.

## Fichiers impactés

| Fichier | Fonction |
|---------|----------|
| `src/ui/graph_view.rs` | `build_commit_line()`, constante `COL_SPACING` |
| `src/utils/time.rs` | `format_relative_time()` |

## Solutions proposées

### 5.1 — Espacement adaptatif

Ne pas changer la constante `COL_SPACING` (2 est un bon compromis pour l'espace horizontal), mais ajouter un **séparateur visuel** entre la zone graphe et la zone texte.

### 5.2 — Troncature intelligente du message

Calculer la largeur disponible pour le message en fonction de la largeur du terminal, et tronquer avec des `…` si nécessaire.

```rust
// src/ui/graph_view.rs — build_commit_line()
// Paramètre supplémentaire : la largeur totale disponible
fn build_commit_line(row: &GraphRow, is_selected: bool, available_width: u16) -> Line<'static> {
    // ...
    
    // Calculer l'espace déjà utilisé par le graphe, le hash, les refs, l'auteur, la date
    let graph_width = num_cols * COL_SPACING;
    let hash_width = 8;  // "abc1234 "
    let refs_width: usize = node.refs.iter().map(|r| r.len() + 3).sum(); // "[ref] "
    let author_date = format!(" — {} ({})", node.author, format_relative_time(node.timestamp));
    let author_date_width = author_date.len();
    
    let overhead = graph_width + 1 + hash_width + refs_width + author_date_width;
    let max_message_width = (available_width as usize).saturating_sub(overhead);
    
    // Tronquer le message si nécessaire
    let display_message = if node.message.len() > max_message_width && max_message_width > 3 {
        format!("{}…", &node.message[..max_message_width - 1])
    } else {
        node.message.clone()
    };
    
    spans.push(Span::styled(display_message, message_style));
    // ...
}
```

**Note** : Il faut passer `area.width` depuis `render()` → `build_graph_items()` → `build_commit_line()`.

### 5.3 — Séparateur graphe/texte

Ajouter un caractère de séparation subtil entre le graphe et le contenu textuel :

```rust
// Après les colonnes du graphe, avant le hash :
spans.push(Span::styled(
    " ",  // ou "▏" pour un séparateur visuel fin
    Style::default().fg(theme.border_inactive),
));
```

Alternative plus simple : ajouter 2 espaces au lieu de 1 pour créer un gap visuel naturel.

### 5.4 — Différencier auteur et date

Séparer l'auteur et la date en deux spans avec des styles légèrement différents :

```rust
// Avant :
spans.push(Span::styled(
    format!(" — {} ({})", node.author, relative_date),
    Style::default().fg(theme.text_secondary),
));

// Après :
spans.push(Span::styled(
    format!(" — {}", node.author),
    sel_style(theme.text_secondary),
));
spans.push(Span::styled(
    format!(" {}", relative_date),
    sel_style(theme.text_secondary).add_modifier(Modifier::DIM),
));
```

### 5.5 (bonus) — Aligner les messages de commit

Pour une meilleure lisibilité, aligner tous les messages de commit au même niveau horizontal. Calculer la largeur maximale du graphe (nombre max de colonnes) et padder les lignes avec moins de colonnes :

```rust
// Calculer max_cols sur tout le graphe
let max_cols = graph.iter().map(|r| r.cells.len().max(r.node.column + 1)).max().unwrap_or(1);

// Dans build_commit_line, padder jusqu'à max_cols
for col in num_cols..max_cols {
    spans.push(Span::raw(" ".repeat(COL_SPACING)));
}
```

## Signature de `build_commit_line` modifiée

```rust
fn build_commit_line(
    row: &GraphRow,
    is_selected: bool,
    available_width: u16,
    max_graph_cols: usize,
) -> Line<'static>
```

Et `build_graph_items` :

```rust
fn build_graph_items(
    graph: &[GraphRow],
    selected_index: usize,
    available_width: u16,
) -> Vec<ListItem<'static>>
```

## Résultat visuel attendu

```
Avant :
● abc1234 [main] Fix: correction du bug de sélection dans le graphe qui causait un décalage entre l'index — Alice (il y a 2h)
│ ● def5678 [feature/long-name] Ajout de la fonctionnalité de filtrage avancé avec support des regex et des expressions — Bob (il y a 1h)

Après :
●    abc1234 [main]         Fix: correction du bug de sélection dans le graph…  — Alice   il y a 2h
│ ●  def5678 [feature/long] Ajout de la fonctionnalité de filtrage avancé av…   — Bob     il y a 1h
```

## Tests à ajouter

```rust
#[test]
fn test_message_truncation() {
    let mut row = create_test_graph()[0].clone();
    row.node.message = "A".repeat(200);
    let line = build_commit_line(&row, false, 80, 2);
    let total_len: usize = line.spans.iter().map(|s| s.content.len()).sum();
    assert!(total_len <= 80, "La ligne ne doit pas dépasser 80 chars");
}

#[test]
fn test_graph_columns_aligned() {
    let graph = create_multi_branch_graph(); // Helper avec branches variées
    let items = build_graph_items(&graph, 0, 120);
    // Vérifier que le hash commence toujours à la même position
}
```

## Critère de validation

- Les messages longs sont tronqués avec `…` au lieu d'être coupés.
- L'auteur et la date restent toujours visibles.
- Les colonnes de texte sont alignées verticalement.
- `cargo test` passe.
