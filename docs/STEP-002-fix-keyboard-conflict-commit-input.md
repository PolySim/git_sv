# STEP-002 — Bug : Touches 1/2/3 en conflit avec la saisie de commit

## Problème

Lors de la saisie d'un message de commit (vue Staging, focus `CommitMessage`), taper les chiffres `1`, `2` ou `3` déclenche les raccourcis de changement de vue (`SwitchToGraph`, `SwitchToStaging`, `SwitchToBranches`) au lieu d'insérer le caractère dans le message.

## Cause

Dans `src/ui/input.rs`, la navigation entre vues (lignes 76-86) est traitée **avant** le dispatch vers `map_staging_key()` (ligne 94). Or `map_staging_key()` gère le mode `CommitMessage` en priorité (ligne 280), mais elle n'est jamais atteinte pour les touches `1`, `2`, `3` car elles sont interceptées plus haut :

```rust
// Ligne 76 — TOUJOURS exécuté, même en mode saisie
match key.code {
    KeyCode::Char('1') => return Some(AppAction::SwitchToGraph),
    KeyCode::Char('2') => return Some(AppAction::SwitchToStaging),
    KeyCode::Char('3') => return Some(AppAction::SwitchToBranches),
    // ...
}

// Ligne 94 — jamais atteint pour '1', '2', '3'
if state.view_mode == ViewMode::Staging {
    return map_staging_key(key, state);
}
```

Le meme probleme se pose pour le mode Input dans la vue Branches (`BranchesFocus::Input`).

## Fichiers concernés

- `src/ui/input.rs` — `map_key()` (lignes 76-86)

## Solution

Ajouter une vérification des modes de saisie **avant** le bloc de navigation entre vues. Si l'utilisateur est en train de saisir du texte, ne pas intercepter les touches :

```rust
fn map_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // Ctrl+C quitte toujours
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(AppAction::Quit);
    }

    // Merge picker...
    // Confirmation en attente...
    // Recherche active...

    // NOUVEAU: Si on est en mode saisie de texte, dispatch immédiatement
    // vers le handler de la vue concernée (pas de raccourcis globaux)
    if state.view_mode == ViewMode::Staging
        && state.staging_state.focus == StagingFocus::CommitMessage
    {
        return map_staging_key(key, state);
    }
    if state.view_mode == ViewMode::Branches
        && state.branches_view_state.focus == BranchesFocus::Input
    {
        return map_branches_key(key, state);
    }

    // Navigation entre vues (1/2/3/4) — seulement si pas en saisie
    match key.code {
        KeyCode::Char('1') => return Some(AppAction::SwitchToGraph),
        // ...
    }
    // ...
}
```

## Tests

- En vue Staging, saisir un message contenant "fix #123" et vérifier que les chiffres s'insèrent
- En vue Branches, créer une branche nommée "v2.1" et vérifier que les chiffres s'insèrent
- Vérifier que les touches 1/2/3 fonctionnent toujours pour changer de vue quand on n'est pas en mode saisie
