# Phase 7 — Fonctionnalités manquantes (TODO existants)

## 7.1 Implémenter `BranchCreate` dans la vue Graph

**Fichier** : `app.rs:549`

- [ ] Ouvrir un overlay d'input pour saisir le nom de la nouvelle branche.
- [ ] Réutiliser le mécanisme `InputAction` déjà en place dans la vue Branches.

## 7.2 Implémenter `BranchDelete` dans la vue Graph

**Fichier** : `app.rs:552`

- [ ] Ajouter une confirmation avant suppression.
- [ ] Empêcher la suppression de la branche courante (HEAD).

## 7.3 Implémenter les prompts interactifs

**Fichier** : `app.rs:928`

- [ ] `CommitPrompt` : Rediriger vers la vue Staging avec le focus sur le commit message.
- [ ] `StashPrompt` : Ouvrir un overlay pour saisir le message du stash.
- [ ] `MergePrompt` : Ouvrir un overlay pour choisir la branche à merger.
