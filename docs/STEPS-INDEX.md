# Plan d'amélioration UI/UX du Git Graph

## Vue d'ensemble

Ce plan couvre la correction de bugs et l'amélioration de l'UI/UX de la vue graphe de `git_sv`.
Les STEPs sont classés par priorité et doivent être implémentés dans l'ordre.

## STEPs

### Bugs (priorité haute)

| STEP | Fichier | Résumé | Fichiers principaux |
|------|---------|--------|---------------------|
| [01](STEP-01-fix-selection-sync.md) | Bug critique | La formule `selected_index * 2` pour la sélection visuelle est incorrecte car le dernier commit n'a pas de `ConnectionRow` | `graph_view.rs`, `navigation.rs`, `state/mod.rs` |
| [02](STEP-02-fix-horizontal-lines.md) | Bug visuel | `find_horizontal_color()` vérifie toute la rangée et dessine des lignes horizontales parasites au-delà des points d'arrivée | `graph_view.rs` |
| [03](STEP-03-fix-selection-highlight.md) | Bug UX | Seul le message du commit reçoit le style de sélection, pas le hash, les refs, l'auteur ni la date | `graph_view.rs` |

### Améliorations UX (priorité moyenne)

| STEP | Fichier | Résumé | Fichiers principaux |
|------|---------|--------|---------------------|
| [04](STEP-04-column-compaction.md) | Compaction | Les colonnes libérées du graphe ne sont jamais supprimées, le graphe s'élargit indéfiniment | `git/graph.rs` |
| [05](STEP-05-graph-readability.md) | Lisibilité | Troncature des messages, alignement des colonnes de texte, différenciation auteur/date | `graph_view.rs` |
| [06](STEP-06-branch-labels.md) | Labels | Pas de distinction entre branches locales, remotes, tags et HEAD | `graph_view.rs`, `git/graph.rs` |
| [07](STEP-07-detail-panel.md) | Détail | Couleurs hardcodées, pas de séparateur, pas d'indicateur de type de commit | `detail_view.rs` |

### Dette technique (priorité basse)

| STEP | Fichier | Résumé | Fichiers principaux |
|------|---------|--------|---------------------|
| [08](STEP-08-cleanup-legacy-variants.md) | Nettoyage | Variants legacy dupliqués dans `FocusPanel` et `BottomLeftMode` | `state/view/mod.rs`, tous les handlers |

## Dépendances entre STEPs

```
STEP-01 ──→ STEP-03 (la sélection doit être correcte avant d'améliorer son style)
STEP-02 (indépendant)
STEP-04 ──→ STEP-05 (la compaction réduit la largeur, impacte la troncature)
STEP-06 (indépendant, mais impacte STEP-05 pour le calcul de largeur)
STEP-07 (indépendant)
STEP-08 (indépendant, peut être fait à tout moment)
```

## Ordre d'implémentation recommandé

1. **STEP-01** — Fix sélection (bug critique)
2. **STEP-02** — Fix lignes horizontales (bug visuel)
3. **STEP-03** — Fix highlight sélection (UX critique)
4. **STEP-08** — Nettoyage legacy (simplifie le code pour la suite)
5. **STEP-04** — Compaction colonnes
6. **STEP-05** — Lisibilité graphe
7. **STEP-06** — Labels branches/tags
8. **STEP-07** — Panneau détail

## Architecture des fichiers concernés

```
src/
├── git/
│   └── graph.rs          ← STEP-01, 02, 04, 06
├── ui/
│   ├── graph_view.rs     ← STEP-01, 02, 03, 05, 06
│   ├── detail_view.rs    ← STEP-07
│   ├── theme.rs          ← Référence thème (pas de modif)
│   └── common/style.rs   ← STEP-07
├── state/
│   ├── mod.rs            ← STEP-01, 08
│   └── view/mod.rs       ← STEP-08
└── handler/
    └── navigation.rs     ← STEP-01, 08
```
