# STEP 08.2 - Diff side-by-side

**Priorite**: Haute  
**Effort estime**: 4h  
**Impact**: Eleve

## Objectif

Ajouter un mode d'affichage des diffs en deux colonnes (ancien vs nouveau), en plus du mode unified existant.

## Portee

- Nouveau mode `SideBySide`.
- Toggle de mode dans la vue diff (ex: `Tab` ou `v`).
- Scroll synchronise des deux colonnes.
- Fallback automatique en mode unified si largeur terminal insuffisante.

## Fichiers impactes (proposition)

- `src/ui/diff_view.rs`
- `src/state.rs` (etat du mode diff)
- `src/event.rs` (raccourci de switch)

## Plan d'implementation

1. Introduire `DiffViewMode { Unified, SideBySide }`.
2. Normaliser les hunks en paires de lignes (left/right) pour le rendu.
3. Rendre deux panes 50/50 avec line numbers et marqueurs `+/-`.
4. Gerer les lignes absentes via placeholders (`""` cote oppose).
5. Conserver les highlights actuels (ajout/suppression) dans les deux modes.
6. Persister le dernier mode choisi en memoire de session.

## Tests

- Unitaires sur l'alignement de hunks vers paires side-by-side.
- Test de rendu sur diff simple, renommage, gros fichiers.
- Test comportement si largeur < seuil (fallback).

## Criteres d'acceptation

- Le switch de mode est instantane et stable.
- Le scroll reste lisible et coherent entre colonnes.
- Le mode unified actuel ne regressse pas.
