# STEP-01 : Thème clair — Texte des commits illisible dans la vue Graph

## Problème

Dans la vue graph, le message des commits utilise `theme.text_normal` qui vaut `Color::White` en thème sombre et `Color::Black` en thème clair. Cependant, **le thème est toujours fixé sur `dark()` par défaut** (`Theme::default()` → `Theme::dark()`), et le `set_theme()` est un no-op.

Le vrai problème est que les couleurs en dur (White, DarkGray, etc.) ne s'adaptent pas au thème du terminal de l'utilisateur. Quand l'utilisateur utilise un terminal avec un fond clair, `Color::White` est quasi-invisible.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/ui/theme.rs` | 38-57 (dark), 60-79 (light) | Définition des thèmes |
| `src/ui/theme.rs` | 115-118 | `THEME` statique, toujours `dark()` |
| `src/ui/graph_view.rs` | 152-158 | Style du message commit (`theme.text_normal`) |
| `src/ui/graph_view.rs` | 143-151 | Style du message sélectionné (`selection_bg/fg`) |
| `src/ui/graph_view.rs` | 164-167 | Style auteur/date (`theme.text_secondary`) |

## Analyse

Dans `graph_view.rs:152-158`, le message du commit non sélectionné est stylé avec :
```rust
Style::default().fg(theme.text_normal)
```

`text_normal` = `Color::White` (thème sombre) ou `Color::Black` (thème clair). Comme le thème est toujours `dark()`, c'est toujours blanc.

## Solution proposée

### Option A : Détection automatique du thème du terminal (recommandée)

1. **Ajouter la dépendance `terminal-light`** dans `Cargo.toml` pour détecter le thème du terminal au démarrage.

2. **Modifier `src/ui/theme.rs`** :
   - Remplacer le `LazyLock` statique par une détection automatique :
     ```rust
     pub static THEME: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(|| {
         match terminal_light::luma() {
             Ok(luma) if luma > 0.5 => Theme::light(),
             _ => Theme::dark(), // Défaut sombre si détection échoue
         }
     });
     ```

3. **Aucune modification nécessaire dans `graph_view.rs`** — les couleurs seront automatiquement correctes via le thème.

### Option B : Utiliser `Color::Reset` / couleurs adaptatives (alternative simple)

1. **Modifier `src/ui/theme.rs`** :
   - Utiliser `Color::Reset` pour `text_normal` afin que le terminal utilise sa couleur de texte par défaut :
     ```rust
     text_normal: Color::Reset,
     ```
   - Cela fonctionne dans tous les terminaux sans dépendance externe.

2. **Inconvénient** : `Color::Reset` ne permet pas de forcer une couleur spécifique — le rendu dépend entièrement du terminal.

### Option C : Argument CLI `--theme light|dark`

1. **Modifier `src/main.rs`** : Ajouter un argument `--theme` via clap.
2. **Modifier `src/ui/theme.rs`** : Passer le choix au `LazyLock` ou utiliser un `OnceLock` initialisé au démarrage.

## Ordre d'implémentation

1. Ajouter la dépendance (si option A)
2. Modifier `src/ui/theme.rs` — détection ou `Color::Reset`
3. Vérifier que `graph_view.rs`, `search_bar.rs`, `filter_popup.rs`, `conflicts_view.rs` et tous les fichiers UI utilisent bien `theme.text_normal` et non des couleurs en dur
4. Tester en thème clair et sombre

## Critère de validation

- Le texte des commits est lisible sur un terminal à fond clair
- Le texte des commits est lisible sur un terminal à fond sombre
- Les éléments sélectionnés restent visuellement distincts
