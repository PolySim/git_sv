# Phase 3 — Qualité du code & correctness

## 3.1 Gestion d'erreurs incohérente ✅

**Problème** : Mélange entre `anyhow::Result` (dans `main.rs`) et `crate::error::Result` (partout ailleurs). Utilisation de `unwrap_or_default()` qui masque les erreurs silencieusement.

- [x] **Supprimer le `expect()` dans `commit.rs:53`** - Remplacé par `ok_or_else(|| GitSvError::Other(...))?` pour une gestion d'erreur propre.
- [ ] **Unifier la gestion d'erreurs** - À faire : Utiliser `crate::error::Result` partout dans le code interne.
- [ ] **Remplacer les `unwrap_or_default()`** - À faire dans une itération future avec gestion des erreurs via flash messages.

## 3.2 Problème de double appel à `status()` ✅

**Problème** : `refresh()` appelle `self.repo.status()` puis immédiatement après `self.refresh_staging()` qui rappelle `self.repo.status()`.

- [x] **Créé `refresh_staging_with_entries()`** - Nouvelle méthode qui accepte les `status_entries` en paramètre.
- [x] **Modifié `refresh()`** - Passe maintenant `&self.state.status_entries` à `refresh_staging_with_entries()`.
- [x] **Gardé `refresh_staging()`** - Pour les cas où on veut explicitement rafraîchir (appelle `status()` puis `refresh_staging_with_entries()`).

**Résultat** : Évite un appel git2 redondant lors du rafraîchissement complet.

## 3.3 Conversion `InsertChar` non-safe avec les caractères multi-byte ✅

**Problème** : `commit_message.insert()` et `commit_message.remove()` utilisent des indices d'octets mais `cursor_position` compte les caractères. Cela peut causer un panic avec des caractères Unicode multi-octets (ex: émojis, accents).

- [x] **Créé `char_to_byte_position()`** - Fonction utilitaire qui convertit une position de caractère en position d'octet via `char_indices()`.
- [x] **Modifié `handle_insert_char()`** - Utilise `char_to_byte_position()` pour trouver l'index d'insertion correct.
- [x] **Modifié `handle_delete_char()`** - Utilise `char_to_byte_position()` pour trouver les bornes du caractère à supprimer avec `drain()`.
- [x] **Modifié `handle_move_cursor_right()`** - Compare maintenant avec `chars().count()` au lieu de `len()`.

**Résultat** : Gestion correcte des caractères Unicode multi-octets (émojis, accents, etc.) dans les messages de commit.

## 3.4 Nettoyage des imports et des `clone()` inutiles ⏳

- [x] Remplacer `bottom_left_mode.clone()` et `focus.clone()` - **Fait implicitement** avec l'ajout de `Copy`.
- [ ] Supprimer les imports non utilisés - À faire avec `cargo clippy`.
- [ ] Remplacer `&Option<String>` par `Option<&str>` - À faire dans une itération future.

## 3.5 Ajout de `Copy` aux enums simples ✅

**Problème** : `ViewMode`, `BottomLeftMode`, `FocusPanel`, `StagingFocus`, `BranchesSection`, `BranchesFocus` implémentent `Clone` mais pas `Copy`. Ce sont des enums sans données associées.

- [x] **Ajouté `Copy`** aux enums suivantes dans `state.rs` :
  - `ViewMode` (Graph, Help, Staging, Branches)
  - `BottomLeftMode` (CommitFiles, WorkingDir)
  - `FocusPanel` (Graph, Files, Detail)
  - `StagingFocus` (Unstaged, Staged, Diff, CommitMessage)
  - `BranchesSection` (Branches, Worktrees, Stashes)
  - `BranchesFocus` (List, Detail, Input)
  - `InputAction` (CreateBranch, CreateWorktree, RenameBranch, SaveStash)

**Résultat** : Les appels `.clone()` sur ces types sont maintenant des copies implicites, plus performantes et idiomatiques.
