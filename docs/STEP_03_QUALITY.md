# Phase 3 — Qualité du code & correctness

## 3.1 Gestion d'erreurs incohérente

**Problème** : Mélange entre `anyhow::Result` (dans `main.rs`) et `crate::error::Result` (partout ailleurs). Utilisation de `unwrap_or_default()` qui masque les erreurs silencieusement.

- [ ] **Unifier la gestion d'erreurs** : Utiliser `crate::error::Result` partout dans le code interne, et `anyhow::Result` uniquement dans `main()`.
- [ ] **Remplacer les `unwrap_or_default()`** dans `refresh()` par une propagation d'erreur avec `?`, ou au minimum logger l'erreur dans un flash message.
- [ ] **Supprimer le `expect()` dans `commit.rs:53`** (`"HEAD devrait pointer vers un commit"`) et le remplacer par un `ok_or_else(|| GitSvError::Other(...))?`.

## 3.2 Problème de double appel à `status()`

**Problème** : `refresh()` appelle `self.repo.status()` puis immédiatement après `self.refresh_staging()` qui rappelle `self.repo.status()`.

- [ ] Passer les `status_entries` déjà récupérés à `refresh_staging()` pour éviter le double appel.

## 3.3 Conversion `InsertChar` non-safe avec les caractères multi-byte

**Problème** : `commit_message.insert()` et `commit_message.remove()` utilisent des indices d'octets mais `cursor_position` compte les caractères. Cela peut causer un panic avec des caractères Unicode multi-octets (ex: émojis, accents).

- [ ] Utiliser `char_indices()` pour convertir la position de caractère en position d'octet.
- [ ] Ou utiliser une crate comme `ropey` pour gérer correctement le texte.

## 3.4 Nettoyage des imports et des `clone()` inutiles

- [ ] Remplacer `bottom_left_mode.clone()` et `focus.clone()` par des copies (ajouter `Copy` aux enums simples).
- [ ] Supprimer les imports non utilisés (vérifier avec `cargo clippy`).
- [ ] Remplacer `&Option<String>` par `Option<&str>` dans les signatures de fonctions (plus idiomatique).

## 3.5 Ajout de `Copy` aux enums simples

**Problème** : `ViewMode`, `BottomLeftMode`, `FocusPanel`, `StagingFocus`, `BranchesSection`, `BranchesFocus` implémentent `Clone` mais pas `Copy`. Ce sont des enums sans données associées.

- [ ] Ajouter `#[derive(Copy)]` à ces enums pour éviter les `.clone()` inutiles.
