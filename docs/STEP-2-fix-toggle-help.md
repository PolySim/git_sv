# STEP-2 : L'overlay d'aide (?) ne s'affiche plus

## Problème

Presser `?` dans n'importe quelle vue ne fait rien. L'overlay d'aide qui devrait s'afficher par-dessus la vue courante est inaccessible.

## Cause racine

`AppAction::ToggleHelp` est dispatché comme un **no-op** dans le dispatcher. Le handler ne change jamais `view_mode` vers `ViewMode::Help`.

### Détail du flux cassé

1. **`input.rs`** mappe `?` vers `AppAction::ToggleHelp` (lignes ~162, ~243, ~281, ~340, ~530) — OK.

2. **`dispatcher.rs:73`** traite `ToggleHelp` ainsi :
   ```rust
   AppAction::ToggleHelp => {
       // Toggle help est géré au niveau de l'UI, pas ici
       Ok(())
   }
   ```
   C'est un **no-op**. Le commentaire dit "géré au niveau de l'UI" mais rien dans l'UI ne gère ça. C'est un reste de l'ancienne architecture où le toggle était fait avant le dispatch.

3. **`ui/mod.rs:48`** vérifie `ViewMode::Help` pour afficher l'overlay, mais ce mode n'est **jamais activé** car aucun code ne fait `state.view_mode = ViewMode::Help`.

4. **`input.rs:161`** vérifie `state.view_mode == ViewMode::Help` pour fermer avec `Esc`, mais puisqu'on ne peut jamais entrer en mode Help, ce code est mort.

## Fichiers concernés

| Fichier | Ligne(s) | Problème |
|---|---|---|
| `src/handler/dispatcher.rs` | ~73-75 | `ToggleHelp` est un no-op |
| `src/ui/mod.rs` | ~48-62 | Rendu conditionnel sur `ViewMode::Help` (jamais atteint) |
| `src/ui/input.rs` | ~161-163 | Esc pour fermer le Help (code mort) |

## Correction proposée

### Remplacer le no-op dans `dispatcher.rs`

```rust
AppAction::ToggleHelp => {
    if ctx.state.view_mode == ViewMode::Help {
        // Retour à la vue précédente (Graph par défaut)
        ctx.state.view_mode = ViewMode::Graph;
    } else {
        ctx.state.previous_view_mode = Some(ctx.state.view_mode);
        ctx.state.view_mode = ViewMode::Help;
    }
    Ok(())
}
```

### Option A (simple) : Retourner toujours vers Graph

Si on ne veut pas ajouter un champ `previous_view_mode` à `AppState`, on peut simplement :

```rust
AppAction::ToggleHelp => {
    ctx.state.view_mode = if ctx.state.view_mode == ViewMode::Help {
        ViewMode::Graph
    } else {
        ViewMode::Help
    };
    Ok(())
}
```

Cela a le défaut de toujours revenir en Graph quand on ferme le Help, même si on l'avait ouvert depuis Staging ou Branches.

### Option B (meilleure) : Ajouter `previous_view_mode` à `AppState`

1. Dans `src/state/mod.rs`, ajouter :
   ```rust
   pub previous_view_mode: Option<ViewMode>,
   ```
   Initialiser à `None` dans `AppState::new()`.

2. Dans `dispatcher.rs` :
   ```rust
   AppAction::ToggleHelp => {
       if ctx.state.view_mode == ViewMode::Help {
           ctx.state.view_mode = ctx.state.previous_view_mode.take().unwrap_or(ViewMode::Graph);
       } else {
           ctx.state.previous_view_mode = Some(ctx.state.view_mode);
           ctx.state.view_mode = ViewMode::Help;
       }
       Ok(())
   }
   ```

### Adapter le rendu pour les vues Staging et Branches

Actuellement `ui/mod.rs:48` ne gère le Help overlay que pour Graph et Conflicts :

```rust
ViewMode::Help => {
    if state.conflicts_state.is_some() {
        // render conflicts + conflicts help overlay
    } else {
        render_graph_view(frame, state);
        help_overlay::render(frame, frame.area());
    }
}
```

Il faudrait aussi supporter le Help depuis Staging et Branches. Si on implémente l'option B, on peut utiliser `previous_view_mode` :

```rust
ViewMode::Help => {
    // Rendre la vue sous-jacente d'abord
    match state.previous_view_mode {
        Some(ViewMode::Staging) => {
            staging_view::render(frame, &state.staging_state, ...);
        }
        Some(ViewMode::Branches) => {
            branches_view::render(frame, &state.branches_view_state, ...);
        }
        Some(ViewMode::Conflicts) | _ if state.conflicts_state.is_some() => {
            if let Some(ref cs) = state.conflicts_state {
                conflicts_view::render(frame, cs, ...);
            }
            conflicts_view::render_help_overlay(frame, frame.area());
            return; // L'overlay de conflits est spécifique
        }
        _ => {
            render_graph_view(frame, state);
        }
    }
    help_overlay::render(frame, frame.area());
}
```

## Vérification

1. `cargo build` compile
2. Lancer l'app, presser `?` → l'overlay d'aide apparaît par-dessus le graphe
3. Presser `Esc` ou `?` → retour à la vue précédente
4. Aller en Staging (`2`), presser `?` → aide visible, `Esc` → retour en Staging
5. Aller en Branches (`3`), presser `?` → aide visible, `Esc` → retour en Branches
