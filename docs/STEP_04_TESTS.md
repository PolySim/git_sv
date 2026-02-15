# Phase 4 — Tests

## 4.1 Infrastructure de tests ✅

**Infrastructure créée** :
- [x] **Ajout de `tempfile`** en dev-dependency dans `Cargo.toml`
- [x] **Créé `src/git/tests/test_utils.rs`** - Helpers pour créer des repos git temporaires
  - `create_test_repo()` - Crée un repo avec branche "main" configurée
  - `create_file()` - Crée un fichier dans le repo
  - `commit()` - Commit les changements de l'index
  - `commit_file()` - Crée un fichier et le commit en une seule opération
- [x] **Créé `tests/common/mod.rs`** - Helpers pour les tests d'intégration (structure prête)

## 4.2 Tests unitaires ✅

### git/commit.rs (7 tests)
- [x] `test_commit_info_from_git2_commit()` - Vérifie la création de CommitInfo
- [x] `test_commit_info_short_hash()` - Vérifie le hash court (7 caractères)
- [x] `test_stage_file()` - Stage un fichier dans l'index
- [x] `test_stage_all()` - Stage tous les fichiers modifiés
- [x] `test_unstage_file()` - Unstage un fichier (retour à HEAD)
- [x] `test_create_commit()` - Crée un commit avec message

### git/repo.rs (8 tests)
- [x] `test_status_entry_is_staged()` - Vérifie is_staged() et is_unstaged()
- [x] `test_status_entry_display_status()` - Vérifie les labels de statut
- [x] `test_git_repo_open()` - Ouvre un repository
- [x] `test_git_repo_current_branch()` - Récupère la branche courante
- [x] `test_git_repo_log()` - Liste les commits
- [x] `test_git_repo_status()` - Récupère le statut du working directory
- [x] `test_git_repo_branches()` - Liste les branches

### git/branch.rs (6 tests)
- [x] `test_list_branches()` - Liste les branches locales
- [x] `test_list_all_branches()` - Liste locales et remotes
- [x] `test_create_branch()` - Crée une nouvelle branche
- [x] `test_checkout_branch()` - Change de branche
- [x] `test_delete_branch()` - Supprime une branche
- [x] `test_rename_branch()` - Renomme une branche

### git/diff.rs (6 tests)
- [x] `test_diff_status_display_char()` - Caractères de statut (A/M/D/R)
- [x] `test_commit_diff_simple()` - Diff d'un commit avec ajout
- [x] `test_commit_diff_multiple_files()` - Diff avec plusieurs fichiers
- [x] `test_commit_diff_modification()` - Diff avec modification
- [x] `test_get_file_diff()` - Diff détaillé d'un fichier
- [x] `test_working_dir_file_diff()` - Diff avec le working directory

### git/graph.rs (5 tests)
- [x] `test_build_graph_linear()` - Graphe avec historique linéaire
- [x] `test_find_or_assign_column()` - Attribution des colonnes
- [x] `test_assign_new_column_reuse()` - Réutilisation des colonnes libres
- [x] `test_determine_color_index()` - Attribution des couleurs
- [x] `test_collect_refs()` - Collection des références

### git/stash.rs (6 tests)
- [x] `test_extract_branch_from_message()` - Extraction de branche du message
- [x] `test_save_stash()` - Sauvegarde d'un stash
- [x] `test_list_stashes()` - Liste des stashes
- [x] `test_apply_stash()` - Application d'un stash
- [x] `test_drop_stash()` - Suppression d'un stash

## 4.3 Résultats

**Total : 35 tests unitaires** - Tous passent ✅

```bash
$ cargo test
test result: ok. 35 passed; 0 failed; 0 ignored
```

## Notes

- Les tests utilisent des repositories git temporaires créés avec `tempfile::TempDir`
- La branche initiale est configurée comme "main" pour la cohérence
- Les tests de stash nécessitent que les fichiers soient staged avant d'être stashés
- Les tests graph testent la logique de placement en colonnes et les couleurs
