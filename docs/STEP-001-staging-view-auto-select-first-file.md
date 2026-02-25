# STEP-001 : Vue Commit — Sélectionner automatiquement le premier fichier et afficher son diff

## Problème

Quand on arrive dans la vue Staging (touche `2` ou `c`), si des fichiers unstaged existent,
le panneau diff affiche « Sélectionnez un fichier pour voir le diff » au lieu de montrer
directement le diff du premier fichier de la liste.

**Cause racine :** `current_diff` est initialisé à `None` et `load_staging_diff()` n'est
jamais appelé à l'entrée dans la vue. Le diff ne se charge que sur un Tab (changement de
focus) ou une action de staging (stage/unstage/discard).  
De plus, la navigation haut/bas (`handle_staging_navigation`) ne recharge **pas** le diff
non plus.

## Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/handler/staging.rs` | `load_staging_diff()` (ligne ~204), `refresh_staging_with_entries()` (ligne ~169) |
| `src/handler/navigation.rs` | `handle_staging_navigation()` (ligne ~154) — ne recharge pas le diff |
| `src/handler/dispatcher.rs` | `AppAction::SwitchToStaging` (ligne ~270) — ne charge pas le diff |
| `src/handler/git.rs` | `handle_commit_prompt()` (ligne ~247) — entre en staging sans charger le diff |
| `src/handler/mod.rs` | `refresh()` (lignes ~145-161) — repopule les listes mais ne charge pas le diff |
| `src/state/view/staging.rs` | `StagingState` — `current_diff` défaut à `None` |
| `src/ui/diff_view.rs` | Message placeholder (lignes 51, 131) |

## Plan de correction

### 1. Charger le diff à l'entrée dans la vue Staging

Dans `src/handler/dispatcher.rs`, après le `SwitchToStaging` (ligne ~270) :

```rust
AppAction::SwitchToStaging => {
    ctx.state.view_mode = ViewMode::Staging;
    ctx.state.dirty = true;
    // AJOUT : charger le diff du premier fichier sélectionné
    crate::handler::staging::load_staging_diff(&mut ctx.state)?;
}
```

Faire la même chose dans `handle_commit_prompt()` (src/handler/git.rs, ligne ~247) après le changement de vue.

### 2. Charger le diff à l'entrée après un refresh

Dans `src/handler/mod.rs`, à la fin du bloc de refresh staging (lignes ~145-161),
ajouter un appel à `load_staging_diff()` si le `view_mode` courant est `Staging`.

### 3. Recharger le diff à la navigation haut/bas

Dans `src/handler/navigation.rs`, dans `handle_staging_navigation()` (ligne ~154),
après la mise à jour du `selected_index` :

```rust
// Après select_next() ou select_previous()
crate::handler::staging::load_staging_diff(state)?;
```

### 4. Vérification

- [ ] Ouvrir la vue staging (`2`) avec au moins un fichier unstaged → le diff du premier fichier s'affiche
- [ ] Naviguer avec j/k dans la liste → le diff change en temps réel
- [ ] Entrer via `c` (commit prompt) → le diff s'affiche si des fichiers sont présents
- [ ] Listes vides → le message placeholder s'affiche toujours correctement
