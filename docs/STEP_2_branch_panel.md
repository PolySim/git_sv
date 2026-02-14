# STEP 2 — Panneau de branches & navigation entre branches

## Problème actuel

- La touche `b` est mappée sur `AppAction::BranchList` dans `input.rs`
  mais l'action ne fait rien (`// TODO` dans `app.rs` ligne 117).
- `branch.rs` a les fonctions `list_branches`, `create_branch`, `checkout_branch`,
  `delete_branch` mais elles ne sont jamais appelées depuis l'UI.
- Impossible de voir la liste des branches ou de changer de branche.

## Objectif

Un panneau overlay (popup) qui s'ouvre avec `b` et permet de :
1. Voir toutes les branches locales (et optionnellement remote)
2. Voir quelle branche est active (HEAD)
3. Naviguer dans la liste avec `j/k`
4. `Enter` pour checkout la branche sélectionnée
5. `n` pour créer une nouvelle branche
6. `d` pour supprimer une branche (avec confirmation)
7. `Esc` ou `b` pour fermer le panneau

## Fichiers à créer/modifier

### 1. Créer `src/ui/branch_panel.rs`

Nouveau fichier pour le rendu du panneau overlay de branches :
- Popup centré (60% largeur, 50% hauteur) avec bordure et titre " Branches "
- Liste scrollable des branches
- La branche HEAD est marquée avec `*` et en gras/vert
- La branche sélectionnée est highlight en fond gris

```
┌─────────── Branches ───────────┐
│  * main                        │
│    feature/login               │
│    feature/dashboard           │
│    fix/typo                    │
│                                │
│  Enter:checkout  n:new  d:del  │
└────────────────────────────────┘
```

### 2. Modifier `src/app.rs` — Ajouter l'état du panneau branches

Ajouter dans `App` :
```rust
pub branches: Vec<BranchInfo>,
pub branch_selected: usize,
pub show_branch_panel: bool,
```

Ajouter les actions :
```rust
AppAction::BranchList       // Toggle le panneau
AppAction::BranchCheckout   // Checkout la branche sélectionnée
AppAction::BranchCreate     // Prompt pour nom de nouvelle branche
AppAction::BranchDelete     // Supprime avec confirmation
```

Dans `apply_action` pour `BranchList` :
- Charger la liste via `self.repo.branches()`
- Basculer `show_branch_panel`

Pour `BranchCheckout` :
- Appeler `checkout_branch()` 
- Fermer le panneau
- Appeler `refresh()` pour recharger le graphe

### 3. Modifier `src/ui/input.rs` — Keybindings contextuels

Le mapping des touches doit dépendre du contexte :
- Si `app.show_branch_panel == true` : les touches `j/k/Enter/d/n/Esc` contrôlent le panneau
- Sinon : comportement normal (navigation du graphe)

Ajouter un `match` sur le contexte actif avant de mapper les touches.

### 4. Modifier `src/ui/mod.rs` — Rendre le panneau par-dessus

Après le rendu normal, si `app.show_branch_panel` :
```rust
if app.show_branch_panel {
    branch_panel::render(frame, app, frame.area());
}
```

Le panneau est dessiné **par-dessus** le layout normal (popup overlay).

### 5. Ajouter à `src/ui/mod.rs`

```rust
pub mod branch_panel;
```

## Critère de validation

- `b` ouvre/ferme le panneau de branches.
- On peut naviguer et checkout une branche.
- Après checkout, le graphe se rafraîchit et affiche la nouvelle branche active.
- Le panneau affiche clairement quelle branche est active.
