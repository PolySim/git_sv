# STEP-07 : Amélioration du panneau de détail

## Priorité : MOYENNE (UX)

## Problèmes identifiés

### 7.1 — Le panneau de détail n'utilise pas le thème global

Le panneau `detail_view.rs` utilise des couleurs codées en dur (`Color::Cyan`, `Color::Yellow`, `Color::Green`, `Color::DarkGray`) au lieu du thème (`current_theme()`). Si l'utilisateur est sur un terminal clair, les couleurs seront incohérentes avec le reste de l'UI.

### Code actuel

```rust
// src/ui/detail_view.rs
let border_style = if is_focused {
    Style::default().fg(Color::Cyan)   // ← Hardcodé
} else {
    Style::default()                     // ← Pas de couleur de bordure inactive
};

// ...
Span::styled(oid_str, Style::default().fg(Color::Yellow)),       // ← Hardcodé
Span::styled(refs_display, Style::default().fg(Color::Cyan)),    // ← Hardcodé
Span::styled(branch.clone(), Style::default().fg(Color::Green)), // ← Hardcodé
Span::styled(parents_str, Style::default().fg(Color::DarkGray)), // ← Hardcodé
```

### 7.2 — Le layout du panneau est basique

Le panneau affiche les informations de manière séquentielle sans hiérarchie visuelle :
```
Commit:  abc1234...
Auteur:  Alice
Date:    2024-01-15 14:30:00
Refs:    main, origin/main
Branche: main
Parents: def5678

Fix: correction du bug
```

Il manque :
- Un séparateur visuel entre les métadonnées et le message.
- Le message complet (multi-ligne) avec une meilleure mise en forme.
- Un indicateur visuel du type de commit (merge, normal).

### 7.3 — Le panneau n'est pas scrollable

Si le message de commit est long (multi-ligne), il est tronqué par la hauteur du panneau. Il n'y a pas de scroll.

## Fichiers impactés

| Fichier | Fonction |
|---------|----------|
| `src/ui/detail_view.rs` | `render()` — rendu complet |
| `src/ui/common/style.rs` | Styles réutilisables |

## Solution proposée

### Étape 1 : Utiliser le thème global

```rust
// src/ui/detail_view.rs

use crate::ui::theme::current_theme;
use crate::ui::common::style::border_style;

pub fn render(
    frame: &mut Frame,
    graph: &[GraphRow],
    selected_index: usize,
    area: Rect,
    is_focused: bool,
) {
    let theme = current_theme();

    // ...

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled("Commit:  ", Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::BOLD)),
            Span::styled(oid_str, Style::default().fg(theme.commit_hash)),
        ]),
        Line::from(vec![
            Span::styled("Auteur:  ", Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::BOLD)),
            Span::styled(author, Style::default().fg(theme.text_normal)),
        ]),
        Line::from(vec![
            Span::styled("Date:    ", Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::BOLD)),
            Span::styled(date_str, Style::default().fg(theme.text_normal)),
        ]),
    ];

    // Refs avec style thème
    if has_refs {
        lines.push(Line::from(vec![
            Span::styled("Refs:    ", Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::BOLD)),
            Span::styled(refs_display, Style::default().fg(theme.primary)),
        ]));
    }

    // Branche
    if let Some(branch) = &node.branch_name {
        lines.push(Line::from(vec![
            Span::styled("Branche: ", Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::BOLD)),
            Span::styled(branch.clone(), Style::default().fg(theme.success)),
        ]));
    }

    // Parents
    if has_parents {
        lines.push(Line::from(vec![
            Span::styled("Parents: ", Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::BOLD)),
            Span::styled(parents_str, Style::default().fg(theme.text_secondary)),
        ]));
    }

    // Séparateur
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(2) as usize),
        Style::default().fg(theme.border_inactive),
    )));

    // Message (multi-ligne)
    for msg_line in message.lines() {
        lines.push(Line::from(Span::styled(
            msg_line.to_string(),
            Style::default().fg(theme.text_normal),
        )));
    }

    // Bordure thème-aware
    let block = Block::default()
        .title(" Détail ")
        .borders(Borders::ALL)
        .border_style(border_style(is_focused));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
```

### Étape 2 : Indicateur de type de commit

Ajouter un emoji ou symbole pour identifier les merges :

```rust
// Indicateur de type
let commit_type = if node.parents.len() > 1 {
    "⊕ Merge"
} else if node.parents.is_empty() {
    "◆ Initial"
} else {
    "● Commit"
};

lines.insert(0, Line::from(Span::styled(
    commit_type.to_string(),
    Style::default()
        .fg(if node.parents.len() > 1 { theme.info } else { theme.text_secondary })
        .add_modifier(Modifier::BOLD),
)));
```

### Étape 3 (optionnel) : Scroll du message

Si le contenu dépasse la hauteur du panneau, permettre le scroll. Cela nécessite d'ajouter un `detail_scroll_offset` dans `AppState` et de le gérer dans les handlers.

```rust
// Dans AppState :
pub detail_scroll_offset: u16,

// Dans detail_view::render() :
let paragraph = Paragraph::new(lines)
    .block(block)
    .scroll((scroll_offset, 0))
    .wrap(ratatui::widgets::Wrap { trim: false });
```

## Résultat visuel attendu

```
┌─ Détail ──────────────────────┐
│ ● Commit                      │
│ Commit:  abc1234def5678...    │
│ Auteur:  Alice <alice@ex.com> │
│ Date:    2024-01-15 14:30:00  │
│ Refs:    main, origin/main    │
│ Branche: main                 │
│ Parents: def5678              │
│ ──────────────────────────── │
│ Fix: correction du bug de     │
│ sélection dans le graphe      │
│                               │
│ Ce commit corrige le problème │
│ de décalage entre l'index...  │
└───────────────────────────────┘
```

## Tests à ajouter

```rust
#[test]
fn test_detail_view_uses_theme() {
    // Vérifier que le rendu n'utilise aucune couleur hardcodée
    // en comparant avec le thème courant
}

#[test]
fn test_detail_view_merge_indicator() {
    let mut row = create_test_graph()[0].clone();
    row.node.parents = vec![Oid::zero(), Oid::zero()]; // 2 parents = merge
    // Vérifier que le rendu contient "⊕ Merge"
}
```

## Critère de validation

- Toutes les couleurs viennent de `current_theme()`.
- Le panneau est visuellement cohérent en thème clair et sombre.
- Le message multi-ligne s'affiche correctement.
- `cargo test` passe.
