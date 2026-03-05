# STEP 10 — Abort merge accessible depuis toutes les vues

**Priorité** : Haute
**Effort estimé** : Faible
**Impact** : Accessibilité — fonctionnalité existante mais mal exposée

---

## Constat

L'abort merge **existe déjà et fonctionne** dans le code :

- **Git** : `abort_merge()` dans `src/git/conflict.rs` (ligne ~341) — appelle `repo.cleanup_state()` + `repo.checkout_head(force)`
- **Handler** : `handle_abort_merge()` dans `src/handler/conflict.rs` (ligne ~490) — appelle `abort_merge()`, remet en `ViewMode::Staging`, flash message
- **Keybinding** : Touche `A` dans la vue Conflits (`src/ui/input.rs`, ligne ~486)
- **Confirmation** : `ConfirmAction::AbortMerge` existe dans `confirm_dialog.rs` (ligne ~26) et est géré dans `dispatcher.rs` (ligne ~171)

**Le problème** : L'abort n'est accessible QUE depuis la vue Conflits (`ViewMode::Conflicts`). Si l'utilisateur quitte la vue conflits (`q`/`Esc` → `ConflictAction::LeaveView` qui retourne en Staging), il n'a plus aucun moyen d'abort le merge en cours sans y retourner. Pire, si le merge en cours n'a pas été détecté au démarrage (car la détection d'état repo manque — cf. section 5 du constat), l'utilisateur est bloqué.

### Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/git/conflict.rs` | `abort_merge()` (ligne ~341), `has_conflicts()` (ligne ~262) |
| `src/handler/conflict.rs` | `handle_abort_merge()` (ligne ~490) |
| `src/handler/dispatcher.rs` | `handle_confirm_action()` pour `ConfirmAction::AbortMerge` (ligne ~171) |
| `src/ui/confirm_dialog.rs` | `ConfirmAction::AbortMerge` (ligne ~26) |
| `src/ui/input.rs` | Keybindings conflits (ligne ~486), graph (lignes 261-305), staging |
| `src/ui/status_bar.rs` | Barre de statut (pas d'indicateur "MERGING" actuellement) |
| `src/handler/mod.rs` | `refresh()` (ligne ~77) — pas de détection d'état repo au démarrage |
| `src/state/mod.rs` | `AppState` — pas de champ tracking l'état repo |

---

## Actions à mener

### 10.1 — Détecter l'état "merge en cours" au démarrage et au refresh (priorité haute)

Actuellement, l'application ne détecte pas si le repo est en état de merge au démarrage. Il faut :

- Ajouter une fonction de détection dans `src/git/conflict.rs` :
```rust
/// Vérifie si le repository est en état de merge (MERGE_HEAD existe).
pub fn is_merging(repo: &Repository) -> bool {
    repo.path().join("MERGE_HEAD").exists()
}
```

- Ajouter un champ `is_merging: bool` dans `AppState` (`src/state/mod.rs`)
- Dans `refresh()` (`src/handler/mod.rs`, ligne ~77), appeler `is_merging()` et mettre à jour l'état
- Si un merge est détecté au démarrage, proposer à l'utilisateur de reprendre la résolution ou d'abort

### 10.2 — Afficher l'indicateur "MERGING" dans la status bar (priorité haute)

Modifier `src/ui/status_bar.rs` pour afficher un indicateur visuel quand `state.is_merging` est true :

```
 main | MERGING | 3 fichiers en conflit
```

Utiliser un style rouge/jaune + bold pour attirer l'attention.

### 10.3 — Rendre l'abort accessible depuis la vue Graph (priorité haute)

Quand un merge est en cours (`state.is_merging == true`), ajouter un keybinding dans la vue Graph :

**`src/ui/input.rs`** — Dans les keybindings graph (vers ligne 290) :
```rust
// Accessible uniquement quand un merge est en cours
KeyCode::Char('A') => Some(AppAction::Git(GitAction::AbortMerge)),
```

**`src/state/action/git.rs`** — Ajouter :
```rust
pub enum GitAction {
    // ... existants ...
    AbortMerge,  // Annuler le merge en cours
}
```

**`src/handler/git.rs`** — Ajouter :
```rust
fn handle_abort_merge(state: &mut AppState) -> Result<()> {
    if !state.is_merging {
        state.set_flash_message("Aucun merge en cours".to_string());
        return Ok(());
    }
    // Demander confirmation via le dialogue existant
    state.pending_confirmation = Some(ConfirmAction::AbortMerge);
    Ok(())
}
```

Le `ConfirmAction::AbortMerge` est déjà géré dans `dispatcher.rs` — il suffit de s'assurer qu'il fonctionne correctement (vérifier qu'il appelle bien `abort_merge()`, met à jour `is_merging`, et fait `mark_dirty()`).

### 10.4 — Rendre l'abort accessible depuis la vue Staging (priorité haute)

Même logique que pour la vue Graph. Quand un merge est en cours, la touche `A` dans la vue Staging doit aussi permettre l'abort.

**`src/ui/input.rs`** — Dans les keybindings staging :
```rust
KeyCode::Char('A') if state.is_merging => Some(AppAction::Git(GitAction::AbortMerge)),
```

### 10.5 — Mettre à jour les barres d'aide (priorité moyenne)

Quand `is_merging` est true, afficher `A:abort merge` dans la barre d'aide des vues Graph et Staging (en plus de la vue Conflits où c'est déjà le cas).

### 10.6 — Bonus : Détecter aussi les autres états (priorité basse)

Étendre la détection à d'autres états git :
- **Rebase** : `.git/rebase-merge/` ou `.git/rebase-apply/` existe
- **Cherry-pick** : `CHERRY_PICK_HEAD` existe
- **Bisect** : `BISECT_LOG` existe

Pour chaque état, afficher l'indicateur dans la status bar et proposer l'abort correspondant.

---

## Critères de validation

- [ ] L'application détecte un merge en cours au démarrage
- [ ] La status bar affiche "MERGING" quand un merge est en cours
- [ ] `A` fonctionne dans la vue Graph pour abort le merge
- [ ] `A` fonctionne dans la vue Staging pour abort le merge
- [ ] Le dialogue de confirmation s'affiche avant l'abort
- [ ] Après l'abort, le graph se rafraîchit et l'indicateur disparaît
- [ ] `cargo clippy` propre
- [ ] `cargo test` passe
