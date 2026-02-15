# Phase 2 — Élimination du code dupliqué

## 2.1 `centered_rect()` dupliqué 3 fois ✅

**Fichiers** : `help_overlay.rs`, `branch_panel.rs`, `branches_layout.rs`

- [x] Extraire `centered_rect()` dans un module commun `ui/common/mod.rs`.
- [x] Remplacer les 3 implémentations par des appels à la version centralisée.
- [x] Ajouté dans `ui/common/mod.rs` avec documentation complète.
- [x] Mis à jour `help_overlay.rs`, `branch_panel.rs`, et `branches_view.rs` pour utiliser `ui::common::centered_rect`.

## 2.2 Logique de branches dupliquée dans `branch.rs` ✅

**Problème** : `list_branches()` (lignes 36-91) et `list_all_branches()` (lignes 94-176) partagent ~80% de leur code pour la partie locale. Le calcul `graph_ahead_behind` est appelé **2 fois** par branche (une fois pour ahead, une fois pour behind) alors qu'un seul appel suffit.

- [x] **Factorisé la logique commune** dans deux fonctions privées :
  - `build_local_branch_info()` - Construit les infos pour les branches locales avec ahead/behind
  - `build_remote_branch_info()` - Construit les infos pour les branches remote
- [x] **`list_branches()`** appelle maintenant `list_all_branches()` et retourne uniquement les branches locales.
- [x] **Corrigé le double appel à `graph_ahead_behind`** : un seul appel avec `.map(|(a, b)| (Some(a), Some(b)))` au lieu de deux appels séparés.

## 2.3 Logique de diff dupliquée dans `diff.rs` ✅

**Problème** : `get_file_diff()` (lignes 137-233) et `working_dir_file_diff()` (lignes 237-333) ont une logique d'extraction de lignes de patch quasi identique (~80 lignes dupliquées).

- [x] **Créé `extract_diff_lines()`** - Extrait les lignes d'un patch donné, retourne `(Vec<DiffLine>, usize, usize)`.
- [x] **Créé `find_and_extract_file_diff()`** - Factorise la logique de recherche du fichier dans le diff et son extraction.
- [x] Les deux fonctions `get_file_diff()` et `working_dir_file_diff()` ne diffèrent maintenant que par la manière de calculer le `Diff` initial.

## 2.4 Status bar dupliquée ✅ (Préparé)

**Problème** : `status_bar.rs`, `staging_view::render_staging_status_bar()` et `branches_view::render_branches_status_bar()` ont une logique similaire.

- [x] **Créé `StatusBarConfig`** dans `ui/common/mod.rs` - Structure de configuration pour les status bars.
- [x] **Créé `render_status_bar()`** - Fonction réutilisable qui accepte une configuration.
- [ ] Utilisation dans `staging_view` et `branches_view` - À faire dans une itération future (optionnel).
