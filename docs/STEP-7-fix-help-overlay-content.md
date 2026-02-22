# STEP-7 : L'overlay d'aide est incomplet

## Problème

L'overlay d'aide (déclenché par `?`) ne documente qu'une fraction des raccourcis disponibles. De nombreuses commandes importantes manquent.

## Cause racine

Le contenu de `help_overlay.rs` est statique et n'a pas été mis à jour après l'ajout de nouvelles fonctionnalités. Il ne référence que les raccourcis de base.

### Raccourcis actuellement documentés

- Navigation : `j/k`, `g`, `G`
- Actions Git : `c`, `s`, `m`, `b`
- Interface : `Tab`, `r`
- Clipboard : `Shift+clic`, `y`

### Raccourcis **manquants**

| Raccourci | Action | Catégorie |
|---|---|---|
| `P` | Push | Actions Git |
| `p` | Pull | Actions Git |
| `f` | Fetch | Actions Git |
| `x` | Cherry-pick | Actions Git |
| `/` | Recherche | Recherche |
| `n` / `N` | Résultat suivant/précédent | Recherche |
| `F` | Filtre avancé | Interface |
| `B` | Blame | Interface |
| `v` | Toggle mode diff (unified/side-by-side) | Interface |
| `1` / `2` / `3` / `4` | Changer de vue | Navigation |
| `Ctrl+D` / `Ctrl+U` | Page down/up | Navigation |
| `PageUp` / `PageDown` | Page down/up | Navigation |
| `Enter` | Détail du commit / action | Navigation |
| `q` | Quitter | Général |

## Fichier concerné

| Fichier | Fonction |
|---|---|
| `src/ui/help_overlay.rs` | `build_help_content()` |

## Correction proposée

Réécrire `build_help_content()` pour inclure tous les raccourcis :

```rust
fn build_help_content() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        // ── Navigation ──
        section_header("Navigation"),
        separator(),
        key_line("j / ↓", "Commit suivant"),
        key_line("k / ↑", "Commit précédent"),
        key_line("g / Home", "Premier commit"),
        key_line("G / End", "Dernier commit"),
        key_line("Ctrl+D / PgDn", "Page suivante"),
        key_line("Ctrl+U / PgUp", "Page précédente"),
        key_line("Enter", "Détail / action"),
        key_line("Tab", "Basculer panneaux"),
        Line::from(""),

        // ── Vues ──
        section_header("Vues"),
        separator(),
        key_line("1", "Vue Graph"),
        key_line("2", "Vue Staging"),
        key_line("3", "Vue Branches"),
        key_line("4", "Vue Conflits (si actifs)"),
        Line::from(""),

        // ── Actions Git ──
        section_header("Actions Git"),
        separator(),
        key_line("c", "Nouveau commit"),
        key_line("s", "Stash"),
        key_line("m", "Merge"),
        key_line("b", "Panneau branches"),
        key_line("P", "Push"),
        key_line("p", "Pull"),
        key_line("f", "Fetch"),
        key_line("x", "Cherry-pick"),
        key_line("B", "Blame du fichier"),
        Line::from(""),

        // ── Recherche & Filtre ──
        section_header("Recherche & Filtre"),
        separator(),
        key_line("/", "Ouvrir la recherche"),
        key_line("n / N", "Résultat suivant / précédent"),
        key_line("F", "Filtre avancé"),
        Line::from(""),

        // ── Interface ──
        section_header("Interface"),
        separator(),
        key_line("v", "Toggle diff (unified/split)"),
        key_line("r", "Rafraîchir"),
        key_line("y", "Copier dans le clipboard"),
        key_line("q", "Quitter"),
        Line::from(""),

        Line::from(vec![Span::styled(
            "Esc ou ? pour fermer",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]),
    ]
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(vec![Span::styled(
        title.to_string(),
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Yellow),
    )])
}

fn separator() -> Line<'static> {
    Line::from("─".repeat(40))
}

fn key_line(key: &str, desc: &str) -> Line<'static> {
    let padding = 16usize.saturating_sub(key.len());
    Line::from(vec![
        Span::styled(key.to_string(), Style::default().fg(Color::Cyan)),
        Span::raw(format!("{}{}", " ".repeat(padding), desc)),
    ])
}
```

## Vérification

1. `cargo build` compile
2. Lancer l'app, presser `?` → tous les raccourcis sont listés de manière organisée
3. Vérifier que chaque raccourci listé fonctionne réellement
