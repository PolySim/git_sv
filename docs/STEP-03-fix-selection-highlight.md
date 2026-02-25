# STEP-03 : Amélioration du style de sélection du commit

## Priorité : HAUTE (UX critique)

## Problème

Quand un commit est sélectionné dans le graphe, seul le **message** reçoit le style de sélection (fond + texte). Le reste de la ligne (colonnes du graphe, hash, labels de branches, auteur, date) **n'est PAS stylisé**, ce qui crée une incohérence visuelle forte.

### Code actuel

```rust
// src/ui/graph_view.rs — build_commit_line()

// Le style de sélection n'est appliqué qu'au message :
let message_style = if is_selected {
    Style::default()
        .bg(theme.selection_bg)
        .fg(theme.selection_fg)
        .add_modifier(Modifier::BOLD)
} else {
    Style::default().fg(theme.text_normal)
};
spans.push(Span::styled(node.message.clone(), message_style));

// Le hash, les refs, l'auteur et la date gardent leur style normal :
spans.push(Span::styled(format!("{} ", short_hash), Style::default().fg(theme.commit_hash)));
spans.push(Span::styled(format!(" — {} ({})", ...), Style::default().fg(theme.text_secondary)));
```

### Résultat visuel actuel

```
│ ● abc1234 [main] Premier commit — Alice (il y a 2h)
│ ● def5678 [feature] █████ Deuxième commit ████ — Bob (il y a 1h)
│ ● ghi9012          Troisième commit — Charlie (il y a 30min)
```

Le highlight n'est visible que sur "Deuxième commit", pas sur tout le reste de la ligne.

### Double problème avec `highlight_style`

De plus, la `List` a un `highlight_style` défini :
```rust
.highlight_style(
    Style::default()
        .bg(theme.selection_bg)
        .fg(theme.selection_fg)
        .add_modifier(Modifier::BOLD),
)
```

Ce `highlight_style` est appliqué par ratatui sur toute la ligne sélectionnée, mais il entre en **conflit** avec les styles individuels des spans (les couleurs de branches, le jaune du hash, etc. écrasent le `highlight_style`). Le résultat est un mélange incohérent.

## Fichiers impactés

| Fichier | Fonction |
|---------|----------|
| `src/ui/graph_view.rs` | `build_commit_line()`, `render()` |

## Solution proposée

### Approche : Appliquer le style de sélection à TOUS les spans informationnels

Quand `is_selected` est vrai, tous les spans après le graphe doivent recevoir le fond de sélection. Les colonnes du graphe (●, │) peuvent garder leurs couleurs de branche pour rester identifiables.

```rust
// src/ui/graph_view.rs — build_commit_line()

fn build_commit_line(row: &GraphRow, is_selected: bool) -> Line<'static> {
    let theme = current_theme();
    let node = &row.node;
    let commit_color = get_branch_color(node.color_index);

    let mut spans: Vec<Span<'static>> = Vec::new();

    // === Partie graphe (colonnes) — garder les couleurs de branche ===
    // ... (code existant inchangé pour les colonnes)

    // Espace entre le graphe et le contenu.
    spans.push(Span::raw(" "));

    // === Partie informations — appliquer le style de sélection si sélectionné ===

    // Helper pour le style conditionnel
    let sel_style = |base_fg: Color| -> Style {
        if is_selected {
            Style::default()
                .bg(theme.selection_bg)
                .fg(base_fg)  // Garder la couleur de texte d'origine
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(base_fg)
        }
    };

    // Hash du commit.
    let short_hash = &node.oid.to_string()[..7];
    spans.push(Span::styled(
        format!("{} ", short_hash),
        sel_style(theme.commit_hash),
    ));

    // Labels de branches.
    if !node.refs.is_empty() {
        for ref_name in &node.refs {
            let ref_color = get_branch_color(node.color_index);
            let ref_style = if is_selected {
                Style::default()
                    .fg(ref_color)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default()
                    .fg(ref_color)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            };
            spans.push(Span::styled(format!("[{}] ", ref_name), ref_style));
        }
    }

    // Message du commit.
    spans.push(Span::styled(
        node.message.clone(),
        sel_style(if is_selected { theme.selection_fg } else { theme.text_normal }),
    ));

    // Auteur et date.
    let relative_date = format_relative_time(node.timestamp);
    spans.push(Span::styled(
        format!(" — {} ({})", node.author, relative_date),
        sel_style(theme.text_secondary),
    ));

    Line::from(spans)
}
```

### Supprimer le `highlight_style` de la List

Puisque le style de sélection est géré manuellement dans les spans, on peut soit :
- **Supprimer** `highlight_style` de la `List` (recommandé, pour éviter les conflits)
- Ou le garder minimaliste (juste le fond) :

```rust
// src/ui/graph_view.rs — render()
let list = List::new(items)
    .block(...)
    .highlight_style(Style::default()); // Pas de style automatique
```

### Résultat visuel attendu

```
│ ● abc1234 [main] Premier commit — Alice (il y a 2h)
│ ████████████████████████████████████████████████████████
│ ● ghi9012          Troisième commit — Charlie (il y a 30min)
```

La ligne entière (hash + labels + message + auteur + date) est surlignée de manière cohérente, avec les couleurs d'origine préservées sur le fond de sélection.

## Tests à ajouter

```rust
#[test]
fn test_selected_commit_line_all_spans_have_bg() {
    let row = &create_test_graph()[0];
    let line = build_commit_line(row, true);
    
    let theme = current_theme();
    // Vérifier que tous les spans après le graphe ont le bg de sélection
    let info_spans = line.spans.iter().skip_while(|s| {
        // Skipper les spans du graphe (qui n'ont pas de bg)
        s.style.bg.is_none() && !s.content.contains(char::is_alphanumeric)
    });
    
    for span in info_spans {
        if !span.content.trim().is_empty() {
            assert_eq!(span.style.bg, Some(theme.selection_bg),
                "Le span '{}' devrait avoir le fond de sélection", span.content);
        }
    }
}
```

## Critère de validation

- La sélection est visuellement claire : toute la partie informative de la ligne est surlignée.
- Les colonnes du graphe (●, │) gardent leurs couleurs de branche.
- Pas de conflit entre `highlight_style` et les styles de spans.
- `cargo test` passe.
