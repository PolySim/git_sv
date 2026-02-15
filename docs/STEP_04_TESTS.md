# Phase 4 — Tests

## 4.1 Aucun test n'existe actuellement

**Problème** : Zéro test dans le projet (ni unitaire, ni intégration).

**Étapes** :

- [ ] **Tests unitaires pour `git/commit.rs`** :
  - Tester `CommitInfo::from_git2_commit()`, `short_hash()`.
  - Tester `stage_file()`, `unstage_file()`, `stage_all()`, `unstage_all()` avec un repo temporaire.

- [ ] **Tests unitaires pour `git/graph.rs`** :
  - Tester `build_graph()` avec un repo simple (1 branche linéaire).
  - Tester `build_graph()` avec un merge (2 parents).
  - Tester `find_or_assign_column()`, `assign_new_column()`.
  - Tester `determine_color_index()`.

- [ ] **Tests unitaires pour `git/branch.rs`** :
  - Tester `list_branches()`, `create_branch()`, `delete_branch()`, `rename_branch()`.
  - Tester `checkout_branch()`.

- [ ] **Tests unitaires pour `git/diff.rs`** :
  - Tester `commit_diff()` avec un commit simple.
  - Tester `get_file_diff()` avec ajout/suppression/modification.

- [ ] **Tests unitaires pour `git/stash.rs`** :
  - Tester `save_stash()`, `list_stashes()`, `apply_stash()`, `pop_stash()`, `drop_stash()`.

- [ ] **Tests unitaires pour `git/merge.rs`** :
  - Tester fast-forward merge.
  - Tester merge normal.
  - Tester détection de conflits.

- [ ] **Tests unitaires pour `app.rs`** :
  - Tester `apply_action()` pour chaque action.
  - Tester les transitions d'état (navigation, changement de vue).
  - Tester le flash message et son expiration.

- [ ] **Tests unitaires pour `git/repo.rs`** :
  - Tester `StatusEntry::is_staged()`, `is_unstaged()`, `display_status()`.

- [ ] **Helpers de test** :
  - Créer un helper `create_test_repo()` qui crée un repo git temporaire avec quelques commits.
  - Utiliser `tempfile::TempDir` pour les repos de test.

- [ ] **Tests d'intégration** dans `tests/` :
  - Tester le mode non-interactif (`print_log`).
  - Tester le parsing CLI avec différentes combinaisons d'arguments.
