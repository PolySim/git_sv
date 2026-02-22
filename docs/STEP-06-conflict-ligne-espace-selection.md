# STEP-06 : Conflict mode Ligne — Espace pour sélectionner/désélectionner

## Problème

En mode Ligne de la vue Conflits, la touche Espace ne fait rien. Elle devrait permettre de sélectionner ou désélectionner la ligne courante (toggle du checkbox `[x]`/`[ ]`), exactement comme `Enter` le fait actuellement via `ConflictAction::ToggleLine`.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/ui/input.rs` | 315-395 | `map_conflicts_key` — keybindings de la vue conflits |
| `src/handler/conflict.rs` | 478-543 | `handle_toggle_line` — logique de toggle d'une ligne |
| `src/ui/conflicts_view.rs` | 290-330 | Rendu des checkboxes `[x]`/`[ ]` en mode Ligne |

## Analyse

Dans `map_conflicts_key` (`input.rs:315-395`), il n'y a **aucun binding pour `KeyCode::Char(' ')` (espace)**. La touche Espace est simplement ignorée.

Le toggle de ligne est actuellement déclenché par `Enter` → `ConflictEnterResolve` → `handle_enter_resolve` qui en mode `Line` appelle `handle_toggle_line`.

## Solution proposée

1. **Modifier `src/ui/input.rs`** — `map_conflicts_key` : ajouter un binding pour la touche Espace en mode Ligne.

   Ajouter dans le `match key.code` principal (avant le bloc `Enter`) :

   ```rust
   KeyCode::Char(' ') => {
       if resolution_mode == ConflictResolutionMode::Line
           && matches!(
               panel_focus,
               Some(ConflictPanelFocus::OursPanel | ConflictPanelFocus::TheirsPanel)
           )
       {
           Some(AppAction::ConflictToggleLine)
       } else {
           None
       }
   }
   ```

2. Pas besoin de modifier le handler `handle_toggle_line` — il fonctionne déjà correctement.

## Ordre d'implémentation

1. Ajouter le keybinding `Espace` → `ConflictToggleLine` dans `map_conflicts_key`
2. Conditionner au mode `Line` et aux panneaux `Ours`/`Theirs`
3. Tester : en mode Ligne, Espace toggle `[x]` ↔ `[ ]` sur la ligne courante

## Critère de validation

- En mode Ligne, Espace toggle la sélection de la ligne courante
- Le checkbox `[x]` devient `[ ]` et vice versa
- Le panneau Résultat se met à jour en temps réel
- Espace ne fait rien sur le panneau FileList ou ResultPanel
- Espace ne fait rien en mode Block ou File
