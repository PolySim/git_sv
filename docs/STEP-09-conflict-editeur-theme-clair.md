# STEP-09 : Conflict — Texte de l'éditeur illisible en thème clair

## Problème

Dans la vue Conflits, le panneau Résultat en mode édition (`i`/`e`) affiche du texte en blanc qui est illisible sur un terminal à fond clair. Les lignes normales (non-curseur) utilisent `Span::raw(content)` qui hérite de la couleur par défaut du terminal, mais le fond du panneau et les numéros de ligne peuvent poser problème.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/ui/conflicts_view.rs` | 472-506 | `render_edit_line_with_cursor` — rendu d'une ligne éditée avec curseur |
| `src/ui/conflicts_view.rs` | 521-550 | Rendu du buffer en mode édition — lignes normales |
| `src/ui/conflicts_view.rs` | 552-567 | Rendu normal (non-édition) — couleurs ours/theirs avec `Color::Indexed(22)` et `Color::Indexed(17)` |

## Analyse

### Mode édition (lignes 521-550)

Les lignes normales (sans curseur) sont rendues avec :
```rust
Line::from(vec![
    Span::styled(line_num, Style::default().fg(theme.text_secondary)),
    Span::raw(" "),
    Span::raw(content),  // ← Pas de fg explicite, hérite du terminal
])
```

Le `Span::raw(content)` n'a aucune couleur définie, donc il utilise la couleur par défaut du terminal. Sur un terminal sombre c'est du blanc (OK), sur un terminal clair c'est du noir (OK aussi). **Le problème réel vient du Block du panneau** qui peut avoir un fond fixe.

### Curseur (render_edit_line_with_cursor)

```rust
// Curseur inversé
Span::styled(" ", Style::default().bg(theme.selection_fg).fg(theme.selection_bg));
```

En thème sombre : `selection_fg = White`, `selection_bg = DarkGray` → blanc sur gris foncé (OK).
Mais le texte hors curseur dans cette même fonction n'a pas de `fg` explicite non plus.

### Mode non-édition (lignes 552-567)

```rust
LineSource::Ours => Style::default().bg(Color::Indexed(22)).fg(theme.text_normal),
LineSource::Theirs => Style::default().bg(Color::Indexed(17)).fg(theme.text_normal),
```

Les `Color::Indexed(22)` (vert foncé) et `Color::Indexed(17)` (bleu foncé) sont des couleurs 256 qui ne s'adaptent pas au thème du terminal. `theme.text_normal` est `White` (thème sombre) ce qui donne du blanc sur vert/bleu foncé. Sur un thème clair, cela resterait blanc (si le thème n'est pas détecté) ce qui serait encore lisible sur fond foncé, mais incohérent.

## Solution proposée

### 1. Mode édition : ajouter une couleur explicite au texte

**Modifier `src/ui/conflicts_view.rs`** — rendu des lignes en mode édition :

```rust
// Ligne normale (sans curseur)
Line::from(vec![
    Span::styled(line_num, Style::default().fg(theme.text_secondary)),
    Span::raw(" "),
    Span::styled(content.to_string(), Style::default().fg(theme.text_normal)),
])
```

### 2. Mode édition : ligne avec curseur

**Modifier `render_edit_line_with_cursor`** — ajouter `fg(theme.text_normal)` au texte :

```rust
// Texte avant le curseur
spans.push(Span::styled(before, Style::default().fg(theme.text_normal)));

// Texte après le curseur
spans.push(Span::styled(after, Style::default().fg(theme.text_normal)));
```

### 3. Mode non-édition : adapter les couleurs de fond ours/theirs

Remplacer les `Color::Indexed` par des couleurs du thème ou des couleurs qui s'adaptent :

```rust
// Ajouter dans Theme :
pub ours_bg: Color,
pub theirs_bg: Color,

// Thème sombre :
ours_bg: Color::Indexed(22),   // Vert très foncé
theirs_bg: Color::Indexed(17), // Bleu très foncé

// Thème clair :
ours_bg: Color::Indexed(194),  // Vert très clair
theirs_bg: Color::Indexed(189), // Bleu très clair
```

### 4. Dépendance STEP-01

Ce STEP dépend de STEP-01 pour la détection automatique du thème. Sans détection, `theme.text_normal` sera toujours `White` et le problème persistera en thème clair.

## Ordre d'implémentation

1. (Prérequis : STEP-01 pour la détection du thème)
2. Ajouter `fg(theme.text_normal)` au texte en mode édition
3. Ajouter `fg(theme.text_normal)` au texte dans `render_edit_line_with_cursor`
4. Ajouter `ours_bg` / `theirs_bg` dans `Theme` avec des valeurs adaptées
5. Remplacer les `Color::Indexed` en dur par les couleurs du thème
6. Tester en thème clair et sombre

## Critère de validation

- Le texte en mode édition est lisible sur fond clair
- Le texte en mode édition est lisible sur fond sombre
- Le curseur reste visible et contrasté
- Les numéros de ligne sont visibles
- Les couleurs ours/theirs dans le panneau Résultat s'adaptent au thème
