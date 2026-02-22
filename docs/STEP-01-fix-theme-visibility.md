# STEP-01 — Corriger la visibilité du thème (bg/texte confondus)

## Problème

Le fond est sombre comme le texte, on ne voit quasiment rien. La cause racine est triple :

1. **Le thème (`Theme`) est défini mais jamais utilisé.** Le struct `Theme` dans `ui/theme.rs` expose `current_theme()` avec des paires de couleurs cohérentes (`selection_bg` / `selection_fg`, `text_normal`, `text_secondary`, etc.), mais **aucun code de rendu ne le référence**. Toutes les vues utilisent des couleurs hardcodées.

2. **`Color::DarkGray` est utilisé à la fois comme `fg` du texte secondaire ET comme `bg` de sélection.** Résultat : quand une ligne est sélectionnée, le texte auteur/date (`fg: DarkGray`) est rendu sur un fond `bg: DarkGray` → invisible.

3. **`highlight_style()` ne définit pas de `fg`.** Le style `bg(DarkGray) + BOLD` ne force pas de couleur de texte. Les spans qui avaient un `fg` sombre restent sombres sur le fond sombre.

## Fichiers concernés

| Fichier | Problème |
|---------|----------|
| `src/ui/theme.rs` | `Theme` est dead code — jamais lu par le rendu |
| `src/ui/common/style.rs` | `highlight_style()` manque un `fg`, `INACTIVE_COLOR` = `DarkGray` |
| `src/ui/graph_view.rs` | Couleurs hardcodées, pas de `fg` sur le style sélectionné, `DarkGray` comme fg texte secondaire |
| `src/ui/conflicts_view.rs` | Couleurs hardcodées (`Color::Indexed`) |
| `src/ui/filter_popup.rs` | Couleurs hardcodées |
| `src/ui/search_bar.rs` | Couleurs hardcodées |
| `src/ui/status_bar.rs` | Couleurs hardcodées |
| `src/ui/help_bar.rs` | `bg(Color::Black)` hardcodé |
| `src/ui/branch_panel.rs` | `bg(Color::DarkGray)` hardcodé |
| `src/ui/staging_view.rs` | Couleurs hardcodées |
| `src/ui/branches_view.rs` | Couleurs hardcodées |
| `src/ui/diff_view.rs` | Couleurs hardcodées |
| `src/ui/detail_view.rs` | Couleurs hardcodées |
| `src/ui/blame_view.rs` | Couleurs hardcodées |

## Corrections à apporter

### 1. Rendre `highlight_style()` contrasté

Dans `src/ui/common/style.rs` :

```rust
pub fn highlight_style() -> Style {
    let t = current_theme();
    Style::default()
        .bg(t.selection_bg)
        .fg(t.selection_fg) // ← Manquant aujourd'hui
        .add_modifier(Modifier::BOLD)
}
```

### 2. Migrer `graph_view.rs` vers le thème

Remplacer toutes les couleurs hardcodées par des appels à `current_theme()` :

- `fg(Color::Yellow)` pour le hash → `fg(theme.commit_hash)`
- `fg(Color::DarkGray)` pour auteur/date → `fg(theme.text_secondary)`
- `bg(Color::DarkGray)` pour sélection → `bg(theme.selection_bg)` + `fg(theme.selection_fg)`
- `Style::default()` pour le message non sélectionné → `fg(theme.text_normal)`

### 3. Migrer progressivement toutes les vues vers le thème

Pour chaque fichier listé ci-dessus, remplacer les couleurs hardcodées par les champs correspondants de `current_theme()`. Prioriser les vues principales (graph, staging, conflicts).

### 4. Ajuster les couleurs du thème dark si nécessaire

Vérifier que les paires de couleurs dans `Theme::dark()` offrent un contraste suffisant. Par exemple, `text_secondary` devrait être `Color::Gray` plutôt que `Color::DarkGray` pour rester lisible sur un fond sombre.

### 5. Envisager de poser un `bg` explicite sur les widgets racines

Plutôt que de dépendre du fond par défaut du terminal, poser `bg(theme.background)` sur les `Block` principaux de chaque vue. Cela garantit un rendu cohérent quel que soit le thème du terminal.

## Vérification

- `cargo build` compile
- `cargo clippy` sans warning
- Tester visuellement sur un terminal sombre (iTerm2 dark, Alacritty dark)
- Vérifier que le texte secondaire reste lisible sur la ligne sélectionnée
- Vérifier que chaque vue (graph, staging, conflicts, branches, blame, diff) est lisible
