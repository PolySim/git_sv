# STEP 6 - Fix couleur du texte dans le panneau Résultat (compatibilité thème clair)

## Problème

Le texte dans le panneau Résultat est rendu en `Color::White` (hardcodé dans `build_result_content()`, `conflicts_view.rs` ligne ~440). Sur un terminal avec un thème clair (fond blanc), le texte est invisible.

## Contexte

Le problème existe dans le code actuel avant même les modifications du STEP 5. Si le STEP 5 est implémenté en premier, ce fix est partiellement couvert (les lignes avec provenance ours/theirs auront un background foncé), mais les lignes de contexte (`LineSource::Context`) et les marqueurs de conflit restent problématiques.

## Fichier à modifier

| Fichier | Rôle |
|---------|------|
| `src/ui/conflicts_view.rs` | `build_result_content()` + tous les panneaux de la vue conflits |

## Modifications détaillées

### 1. Principe : ne jamais hardcoder `Color::White`

Remplacer tous les `Color::White` par `Color::Reset` dans la vue conflits. `Color::Reset` utilise la couleur par défaut du terminal, qui est noire sur thème clair et blanche sur thème foncé.

### 2. `build_result_content()` (actuellement ligne ~419)

**Avant** :
```rust
Style::default().fg(Color::White)
```

**Après** :
```rust
Style::default()  // Utilise la couleur par défaut du terminal
```

### 3. Bordures des panneaux

Remplacer dans `render_ours_theirs_panels()` et `render_result_panel()` :

```rust
// Avant
Style::default().fg(Color::White)

// Après
Style::default()  // Couleur par défaut
```

Cela concerne les bordures des blocs quand ils ne sont pas focalisés (état par défaut).

### 4. Autres occurrences dans `conflicts_view.rs`

Scanner tout le fichier pour les `Color::White` et les remplacer par `Color::Reset` ou supprimer le `.fg()` :

- `render_files_panel()` : bordure non focalisée → `Style::default()`
- `render_result_panel()` : bordure non focalisée → `Style::default()`
- `build_result_content()` : texte normal → `Style::default()`

### 5. Vérification dans les autres panneaux

Vérifier aussi `build_ours_content()` et `build_theirs_content()`. Les couleurs `Color::Green` et `Color::Blue` sont OK sur thème clair et foncé. Les `Color::DarkGray` pour le contexte sont aussi acceptables.

### 6. Avec le STEP 5 implémenté

Si le STEP 5 est implémenté, les styles de `build_result_content()` deviennent :

```rust
LineSource::Context => Style::default(),                          // Couleur par défaut du terminal
LineSource::Ours => Style::default().bg(Color::Rgb(0, 40, 0)),   // Fond vert foncé, texte par défaut
LineSource::Theirs => Style::default().bg(Color::Rgb(0, 0, 40)), // Fond bleu foncé, texte par défaut
LineSource::ConflictMarker => Style::default()
    .fg(Color::Yellow)
    .add_modifier(Modifier::BOLD),
```

Pour la compatibilité thème clair avec background coloré, on peut ajuster les couleurs RGB pour qu'elles fonctionnent sur les deux thèmes :

```rust
// Thème-safe : utiliser des couleurs suffisamment saturées
LineSource::Ours => Style::default()
    .bg(Color::Rgb(200, 255, 200))   // Vert pâle — thème clair
    // ou
    .bg(Color::Rgb(0, 40, 0))        // Vert foncé — thème foncé
```

**Option recommandée** : détecter le thème du terminal n'est pas fiable. Utiliser des couleurs de background suffisamment contrastées pour les deux cas. Les couleurs `Color::Indexed()` sont un bon compromis :

```rust
LineSource::Ours => Style::default().bg(Color::Indexed(22)),    // Vert foncé (256 colors)
LineSource::Theirs => Style::default().bg(Color::Indexed(17)),  // Bleu foncé (256 colors)
```

## Résultat attendu

- Le texte du panneau Résultat est lisible sur thème clair et foncé.
- Les bordures non focalisées utilisent la couleur par défaut du terminal.
- Les backgrounds colorés (STEP 5) restent visibles sur les deux thèmes.
