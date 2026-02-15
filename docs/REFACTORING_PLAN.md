# Plan de Refactorisation & Améliorations — git_sv

> Document généré le 15 février 2026.
> Ce document décrit l'ensemble des étapes de refactorisation, corrections et nouvelles fonctionnalités identifiées après analyse complète du projet (~3200 lignes de code, 27 fichiers).

---

## Table des matières

1. [Phase 1 — Refactorisation structurelle](./STEP_01_STRUCTURE.md)
2. [Phase 2 — Élimination du code dupliqué](./STEP_02_DEDUPLICATION.md)
3. [Phase 3 — Qualité du code & correctness](./STEP_03_QUALITY.md)
4. [Phase 4 — Tests](./STEP_04_TESTS.md)
5. [Phase 5 — Performance](./STEP_05_PERFORMANCE.md)
6. [Phase 6 — Améliorations UI/UX](./STEP_06_UI_UX.md)
7. [Phase 7 — Fonctionnalités manquantes (TODO existants)](./STEP_07_TODO.md)
8. [Phase 8 — Nouvelles fonctionnalités](./STEP_08_FEATURES.md)
9. [Ordre de priorité recommandé](./STEP_PRIORITY.md)

---

## Métriques actuelles

| Métrique                         | Valeur   |
|----------------------------------|----------|
| Nombre total de fichiers `.rs`   | 27       |
| Lignes de code totales           | ~3 200   |
| Fichier le plus long             | `app.rs` (1 114 lignes) |
| Nombre de tests                  | 0        |
| TODOs dans le code               | 3        |
| Fonctions avec `#[allow(...)]`   | 2        |
| Code dupliqué identifié          | ~250 lignes |
| Dépendances Cargo                | 7        |

---

## Notes complémentaires

- Le graphe d'algorithme de colonnes (`graph.rs`) est bien conçu et relativement propre. C'est le coeur de l'application et il mérite une attention particulière en termes de tests.
- L'architecture `git/` en tant que couche d'abstraction est une bonne idée. Il faudrait juste s'assurer que `GitRepo` ne laisse pas fuiter `repo.repo` (le champ `pub repo: Repository`). Idéalement, tous les accès devraient passer par des méthodes de `GitRepo`.
- Le choix de `git2` (libgit2) est solide pour les performances mais certaines opérations comme push/pull nécessitent une gestion des credentials qui peut être complexe. Envisager l'appel à `git` en CLI pour ces cas.
- Le projet est dans un bon état de fonctionnement. La priorité #1 devrait être les tests et la refactorisation de `app.rs` pour faciliter toute évolution future.
