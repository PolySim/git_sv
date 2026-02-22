# STEP-04 : Recherche filtres avancés — Texte noir sur fond noir

## Problème

Le popup de filtres avancés (`F`) affiche du texte illisible : texte noir sur fond noir (ou proche). Les champs de saisie sont invisibles, aussi bien en thème sombre qu'en thème clair.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/ui/filter_popup.rs` | 131-168 | `render_filter_field` — style des champs |
| `src/ui/theme.rs` | 38-57 | Thème sombre : `selection_bg: DarkGray`, `selection_fg: White` |
| `src/ui/theme.rs` | 60-79 | Thème clair : `selection_bg: Gray`, `selection_fg: Black` |

## Analyse

Dans `render_filter_field` (`filter_popup.rs:148-155`) :

```rust
let (bg_color, fg_color) = if is_selected {
    (theme.selection_bg, theme.selection_fg)     // DarkGray bg + White fg (OK en sombre)
} else {
    (theme.background, theme.text_secondary)     // Black bg + Gray fg → ILLISIBLE
};
```

Le problème est le champ **non sélectionné** :
- **Thème sombre** : `background` = `Color::Black`, `text_secondary` = `Color::Gray` → Gray sur Black = très peu lisible
- **Thème clair** (si implémenté) : `background` = `Color::White`, `text_secondary` = `Color::DarkGray` → DarkGray sur White = OK

De plus, le popup ne fait pas de `Clear` du fond avec la bonne couleur. Le `Clear` widget (ligne 39) efface la zone mais le fond du terminal apparaît, et le texte se retrouve sur un fond inconnu.

Le champ sélectionné utilise `selection_bg` (`DarkGray`) + `selection_fg` (`White`) ce qui est lisible en sombre, mais le fond `DarkGray` du `Block` border peut se confondre.

## Solution proposée

1. **Modifier `src/ui/filter_popup.rs`** — `render_filter_field` :

   ```rust
   let (bg_color, fg_color) = if is_selected {
       (theme.selection_bg, theme.selection_fg)
   } else {
       // Utiliser un fond légèrement distinct au lieu du background pur
       (Color::Reset, theme.text_normal)
   };
   ```

   Ou mieux, ajouter des couleurs dédiées dans le thème pour les champs de formulaire.

2. **Ajouter un fond au popup** : après le `Clear`, ajouter un fond explicite au `Block` :

   ```rust
   let block = Block::default()
       .title(title)
       .borders(Borders::ALL)
       .border_style(border_style)
       .style(Style::default().bg(theme.background));  // ← Ajouter un fond
   ```

3. **Modifier le style de valeur vide** (`filter_popup.rs:157-161`) : le texte `(vide)` est en `text_secondary` sur `bg_color`, s'assurer qu'il est visible :

   ```rust
   let value_style = if value.is_empty() && is_selected {
       Style::default().fg(theme.text_secondary).bg(bg_color)
   } else {
       Style::default().fg(fg_color).bg(bg_color)
   };
   ```

## Ordre d'implémentation

1. Ajouter `.style(Style::default().bg(theme.background))` au `Block` du popup
2. Corriger les couleurs des champs non sélectionnés
3. S'assurer que les champs sélectionnés ont un contraste suffisant
4. Tester en thème sombre et clair

## Critère de validation

- Les champs du popup de filtre sont lisibles sur fond sombre
- Les champs du popup de filtre sont lisibles sur fond clair
- Le champ actif est visuellement distinct du champ inactif
- Le texte `(vide)` est visible dans les champs vides
