# STEP-09 : Améliorer l'éditeur du panneau Résultat

## Problème

L'éditeur du panneau Résultat a deux problèmes :
1. En mode normal (non-édition), `Enter` sélectionne toute une ligne au lieu de ne rien faire ou d'entrer en édition.
2. En mode édition, il n'y a pas de vrai curseur visible — seule la ligne entière est soulignée. L'utilisateur ne voit pas à quelle colonne il se trouve.

## Prérequis

- STEP-03 (scroll fonctionnel dans le panneau Result)

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/ui/conflicts_view.rs` | `render_result_panel()` (~362-438) | Afficher un vrai curseur (colonne) |
| `src/event.rs` | Handlers d'édition (~2660+) | Vérifier le comportement de `Enter` |
| `src/ui/input.rs` | `map_conflicts_key()` | Clarifier le mapping en mode édition |

## Détail des modifications

### 1. `src/ui/conflicts_view.rs` — Affichage du curseur

En mode édition, au lieu de souligner toute la ligne, on doit :
- Afficher la ligne normalement
- À la position `(edit_cursor_line, edit_cursor_col)`, inverser le style du caractère (ou utiliser un bloc `▏` si entre les caractères)

```rust
// Dans render_result_panel(), mode édition :
fn render_edit_line(line: &str, cursor_col: usize, is_cursor_line: bool) -> Line {
    if !is_cursor_line {
        return Line::from(line.to_string());
    }
    
    let mut spans = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    
    // Texte avant le curseur
    if cursor_col > 0 {
        let before: String = chars[..cursor_col.min(chars.len())].iter().collect();
        spans.push(Span::raw(before));
    }
    
    // Caractère sous le curseur (inversé)
    if cursor_col < chars.len() {
        let cursor_char = chars[cursor_col].to_string();
        spans.push(Span::styled(cursor_char, Style::default().bg(Color::White).fg(Color::Black)));
    } else {
        // Curseur en fin de ligne : afficher un espace inversé
        spans.push(Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)));
    }
    
    // Texte après le curseur
    if cursor_col + 1 < chars.len() {
        let after: String = chars[cursor_col + 1..].iter().collect();
        spans.push(Span::raw(after));
    }
    
    Line::from(spans)
}
```

### 2. Mode édition — Numéro de ligne

Ajouter les numéros de ligne en marge gauche pour faciliter la navigation :

```rust
// Format : "  3 | contenu de la ligne"
let line_num = format!("{:>4} │ ", line_index + 1);
spans.insert(0, Span::styled(line_num, Style::default().fg(Color::DarkGray)));
```

### 3. `src/event.rs` — Comportement de `Enter` dans le panneau Result

**En mode normal (non-édition)** : `Enter` sur le panneau Result ne devrait rien faire (ou entrer en mode édition, comme `i`/`e`). Ne pas sélectionner de ligne.

**En mode édition** : `Enter` insère une nouvelle ligne (comportement actuel `ConflictEditNewline`, ce qui est correct).

Vérifier dans `map_conflicts_key()` que le mapping est correct :

```rust
// Panneau Result, NON en édition :
KeyCode::Enter => Some(AppAction::ConflictStartEditing), // Entrer en édition
// OU
KeyCode::Enter => None, // Ne rien faire

// Panneau Result, EN édition :
KeyCode::Enter => Some(AppAction::ConflictEditNewline), // Déjà correct
```

### 4. Scroll automatique en mode édition

Le scroll du panneau Result doit suivre le curseur :

```rust
// Après chaque mouvement du curseur en édition :
fn auto_scroll_result_to_cursor(cs: &mut ConflictsState, panel_height: usize) {
    if cs.edit_cursor_line < cs.result_scroll {
        cs.result_scroll = cs.edit_cursor_line;
    }
    if cs.edit_cursor_line >= cs.result_scroll + panel_height {
        cs.result_scroll = cs.edit_cursor_line - panel_height + 1;
    }
}
```

### 5. Indicateurs visuels du mode édition

- Bordure magenta (déjà fait)
- Titre : `" Résultat [ÉDITION] "` (déjà fait, vérifier)
- Barre d'aide : afficher les raccourcis d'édition (`Esc:Quitter l'édition  ↑↓←→:Curseur  Backspace:Suppr`)

## Tests

- Entrer en mode édition (`i` depuis le panneau Result).
- Vérifier qu'un curseur clignotant/inversé est visible à la position exacte.
- Taper du texte : le curseur avance caractère par caractère.
- Déplacer le curseur avec les flèches : la position change visuellement.
- `Enter` insère une nouvelle ligne, le curseur descend.
- `Backspace` supprime le caractère précédent.
- Scroller dans un long fichier : le curseur reste visible.
