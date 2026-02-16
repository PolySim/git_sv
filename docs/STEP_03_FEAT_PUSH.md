# STEP 03 — Feature : Améliorer le Git Push

## Contexte

Le push existe déjà (`P` en vue Graph), mais il souffre du même bug SSH que le pull (cf. STEP_01). De plus, il n'est accessible que depuis la vue Graph et pourrait être amélioré.

## État actuel

- **Raccourci** : `P` (majuscule) dans la vue Graph uniquement
- **Code** : `src/git/remote.rs` → `push_current_branch()`
- **Problème SSH** : Même callback minimaliste que le pull, ne supporte pas les alias SSH
- **Limitation** : Non accessible depuis les vues Staging et Branches

## Plan d'amélioration

### Étape 1 — Corriger le bug SSH (dépend de STEP_01)

Après la mise en place de `build_remote_callbacks()` dans STEP_01, s'assurer que `push_current_branch()` utilise la nouvelle fonction au lieu du callback inline.

### Étape 2 — Rendre le push accessible depuis toutes les vues

Ajouter le raccourci `P` dans les keybindings des autres vues :

**`map_staging_key()`** — Ajouter dans les raccourcis globaux (après la vérification du CommitMessage, cf. STEP_02) :
```rust
KeyCode::Char('P') => return Some(AppAction::GitPush),
```

**`map_branches_key()`** — Ajouter dans les raccourcis globaux (après la vérification de l'Input) :
```rust
KeyCode::Char('P') => return Some(AppAction::GitPush),
```

### Étape 3 — Gérer le push d'une branche sans upstream

Actuellement, `push_current_branch()` tente de trouver l'upstream configuré et fait un fallback vers `origin`. Ajouter la logique pour :
1. Détecter si la branche n'a pas d'upstream configuré
2. Proposer de pousser vers `origin/<branch_name>` avec `--set-upstream`
3. Afficher un flash message clair : `"Push de 'feature/x' vers origin (upstream configuré)"`

### Étape 4 — Feedback visuel amélioré

- Afficher un message flash pendant le push : `"Push en cours..."` (nécessite un mode async ou thread)
- Après le push réussi, rafraîchir les indicateurs ahead/behind dans la vue branches
- En cas d'erreur, afficher un message explicite (ex: "Push rejeté : pull nécessaire")

### Étape 5 — Ajouter le push dans la help bar

Mettre à jour les barres d'aide de chaque vue pour mentionner `P:push` :
- `src/ui/help_bar.rs`
- `src/ui/branches_view.rs` → `render_branches_help()`

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/git/remote.rs` | Utiliser `build_remote_callbacks()`, gérer upstream manquant |
| `src/ui/input.rs` | Ajouter `P` dans `map_staging_key()` et `map_branches_key()` |
| `src/ui/help_bar.rs` | Ajouter `P:push` dans les aides contextuelles |
| `src/ui/branches_view.rs` | Ajouter `P:push` dans `render_branches_help()` |

## Dépendances

- **STEP_01** : Correction du bug SSH (obligatoire)
- **STEP_02** : Réordonnement des keybindings staging (recommandé, pour éviter les conflits)

## Priorité

**Moyenne** — Le push fonctionne déjà partiellement, l'amélioration apporte du confort.
