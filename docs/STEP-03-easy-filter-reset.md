# STEP-03 — Permettre de reset facilement les filtres

## Problème

Il faut pouvoir réinitialiser facilement les filtres actifs. Actuellement `FilterAction::Clear` existe et est déclenché par `Ctrl+R`, mais **uniquement depuis l'intérieur du popup filtre** (le raccourci est capturé dans le bloc `if state.filter_popup.is_open` de `map_key()`). Il n'y a aucun raccourci accessible depuis la vue principale du graphe pour effacer les filtres sans ouvrir le popup.

## Fichiers concernés

| Fichier | Modification |
|---------|-------------|
| `src/ui/input.rs` | Ajouter un raccourci en vue Graph pour effacer les filtres |
| `src/ui/help_bar.rs` ou `src/ui/graph_view.rs` | Afficher un indicateur visuel quand des filtres sont actifs |
| `src/ui/nav_bar.rs` ou `src/ui/status_bar.rs` | Afficher un badge/indicateur de filtres actifs |
| `src/handler/filter.rs` | Éventuellement ajouter une action dédiée ou réutiliser `Clear` |

## Corrections à apporter

### 1. Ajouter un raccourci `Ctrl+R` (ou `Shift+R`) en vue Graph hors popup

Dans `src/ui/input.rs`, dans la section Graph (hors popup, hors search) :

```rust
// Si des filtres sont actifs, permettre de les effacer
KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL)
    && state.graph_filter.is_active() => {
    Some(AppAction::ClearFilter)
}
```

Ce raccourci ne doit être actif que lorsque des filtres sont effectivement appliqués (`graph_filter.is_active()`).

### 2. Afficher un indicateur visuel de filtres actifs

Dans la barre de navigation ou la status bar, afficher un badge visible quand des filtres sont actifs :

```
 [FILTRÉ] 
```

Ou intégrer dans la barre d'aide en bas :

```
Ctrl+R: effacer filtres | ...
```

Ce texte ne doit apparaître que quand `state.graph_filter.is_active()` est vrai.

### 3. Afficher le nombre de résultats filtrés

Afficher dans la status bar ou le titre du panneau graph :

```
Graph (42/350 commits) [Filtré: auteur, message]
```

Cela permet à l'utilisateur de savoir immédiatement que des filtres sont actifs et combien de commits sont masqués.

## Vérification

- `cargo build` compile
- `cargo clippy` sans warning
- Tester : appliquer un filtre → l'indicateur "filtré" apparaît dans la UI
- Tester : presser `Ctrl+R` depuis la vue graph (sans ouvrir le popup) → les filtres sont effacés, le graphe complet réapparaît
- Tester : sans filtre actif, `Ctrl+R` ne fait rien (pas de flash message intempestif)
