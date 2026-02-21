# STEP-02 : Corriger la couleur des titres de grilles

## Problème

Les titres des panneaux (bordures des `Block`) sont en blanc (`Color::White`). En mode clair (terminal avec fond blanc), ils sont invisibles.

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/ui/conflicts_view.rs` | `render_ours_panel()` (~166-258), `render_theirs_panel()` (~261-357), `render_result_panel()` (~362-438), `render_files_panel()` (~112-163) | Changer la couleur des titres |

## Détail des modifications

### `src/ui/conflicts_view.rs` — Utiliser une couleur adaptative

Actuellement, les titres utilisent probablement `Style::default().fg(Color::White)` ou sont sur un `Block` avec un style blanc.

**Solution** : Utiliser `Color::Reset` ou `Color::default()` pour les titres, ce qui laisse le terminal appliquer la couleur de texte par défaut (noir sur fond clair, blanc sur fond sombre). Alternativement, utiliser la couleur de la bordure déjà définie (jaune pour le panneau actif, gris/default pour les autres).

Pour chaque panneau (`render_files_panel`, `render_ours_panel`, `render_theirs_panel`, `render_result_panel`) :

```rust
// AVANT (problème en mode clair)
let title = Span::styled(" main ", Style::default().fg(Color::White).bold());

// APRÈS (fonctionne en mode clair ET sombre)
let title_style = if is_focused {
    Style::default().fg(Color::Yellow).bold()
} else {
    Style::default().bold() // Utilise la couleur par défaut du terminal
};
let title = Span::styled(" main ", title_style);
```

**Règle** : Le titre suit la même couleur que la bordure du panneau :
- Panneau actif (focus) : `Color::Yellow` + bold
- Panneau inactif : `Style::default()` (couleur par défaut du terminal) + bold
- Panneau en édition : `Color::Magenta` + bold

## Tests

- Tester avec un terminal en thème clair (ex : `export COLORFGBG="0;15"` ou réglage iTerm2/Terminal).
- Vérifier que les titres sont lisibles dans les deux modes.
- Vérifier que le titre du panneau actif reste visuellement distinct (jaune).
