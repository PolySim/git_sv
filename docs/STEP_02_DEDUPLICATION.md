# Phase 2 — Élimination du code dupliqué

## 2.1 `centered_rect()` dupliqué 3 fois

**Fichiers** : `help_overlay.rs`, `branch_panel.rs`, `branches_layout.rs`

- [ ] Extraire `centered_rect()` dans un module commun `ui/common/mod.rs` ou `ui/utils.rs`.
- [ ] Remplacer les 3 implémentations par des appels à la version centralisée.

## 2.2 Logique de branches dupliquée dans `branch.rs`

**Problème** : `list_branches()` (lignes 36-91) et `list_all_branches()` (lignes 94-176) partagent ~80% de leur code pour la partie locale. Le calcul `graph_ahead_behind` est appelé **2 fois** par branche (une fois pour ahead, une fois pour behind) alors qu'un seul appel suffit.

- [ ] Factoriser la logique commune dans une fonction privée `build_branch_info()`.
- [ ] `list_branches()` devrait appeler `list_all_branches()` et filtrer.
- [ ] Corriger le double appel à `graph_ahead_behind` : stocker le résultat du tuple `(ahead, behind)` en une seule fois.

## 2.3 Logique de diff dupliquée dans `diff.rs`

**Problème** : `get_file_diff()` (lignes 137-233) et `working_dir_file_diff()` (lignes 237-333) ont une logique d'extraction de lignes de patch quasi identique (~80 lignes dupliquées).

- [ ] Extraire une fonction `extract_diff_lines(diff: &Diff, file_path: &str) -> Result<FileDiff>` qui factorise l'itération sur les deltas et les hunks.
- [ ] Les deux fonctions ne diffèrent que par la manière de calculer le `Diff` initial.

## 2.4 Status bar dupliquée

**Problème** : `status_bar.rs`, `staging_view::render_staging_status_bar()` et `branches_view::render_branches_status_bar()` ont une logique similaire.

- [ ] Créer un composant `StatusBar` réutilisable avec un titre de vue configurable.
