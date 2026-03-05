# STEP 09 — Reset de commit depuis la vue Graph

**Priorité** : Haute
**Effort estimé** : Moyen
**Impact** : Fonctionnalité git essentielle manquante

---

## Constat

Il n'existe aucune fonctionnalité `git reset` dans l'application. L'utilisateur ne peut pas ramener la branche courante à un commit précédent depuis la vue graph. C'est une opération fondamentale dans tout client git.

Le pattern existant pour les opérations destructives (cherry-pick) utilise un système de confirmation : l'action crée un `ConfirmAction`, affiche un dialogue, et l'opération s'exécute après validation.

### Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/git/commit.rs` | Opérations git sur les commits (cherry-pick, amend, etc.) |
| `src/git/repo.rs` | Wrapper du repository (`GitRepo`) |
| `src/state/action/git.rs` | Enum `GitAction` (ligne 5) |
| `src/ui/input.rs` | Keybindings de la vue graph (lignes 261-305) |
| `src/ui/confirm_dialog.rs` | Enum `ConfirmAction` (ligne 16) + rendu du dialogue |
| `src/handler/git.rs` | `GitHandler` — handlers des actions git |
| `src/handler/dispatcher.rs` | `handle_confirm_action()` — exécution après confirmation (ligne 397) |

---

## Actions à mener

### 9.1 — Implémenter `git reset` côté git (priorité haute)

Ajouter les fonctions de reset dans `src/git/commit.rs` :

```rust
use git2::ResetType;

/// Reset la branche courante vers le commit spécifié.
pub fn reset_to_commit(repo: &Repository, oid: Oid, reset_type: ResetType) -> Result<()> {
    let commit = repo.find_commit(oid)?;
    let object = commit.as_object();
    let mut checkout_builder = git2::build::CheckoutBuilder::new();
    checkout_builder.force(); // Nécessaire pour Hard reset
    repo.reset(object, reset_type, Some(&mut checkout_builder))?;
    Ok(())
}
```

Deux modes à supporter :
- **Soft** (`ResetType::Soft`) : Déplace HEAD, garde les modifications dans l'index (staged)
- **Hard** (`ResetType::Hard`) : Déplace HEAD, réinitialise l'index ET le working directory

Note : `ResetType::Mixed` est déjà utilisé dans `unstage_all()` (ligne 120 de `commit.rs`) pour le unstage.

Exposer via `GitRepo` dans `src/git/repo.rs` :
```rust
pub fn reset_to_commit(&self, oid: Oid, reset_type: ResetType) -> Result<()> {
    commit::reset_to_commit(&self.repo, oid, reset_type)
}
```

### 9.2 — Ajouter l'action et le picker de type (priorité haute)

**`src/state/action/git.rs`** — Ajouter la variante :
```rust
pub enum GitAction {
    // ... existants ...
    ResetPrompt,  // Ouvre le choix du type de reset
}
```

**Enum pour le type de reset** — Créer dans `src/state/action/git.rs` ou un fichier dédié :
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ResetMode {
    Soft,
    Hard,
}
```

**`src/ui/confirm_dialog.rs`** — Ajouter les variantes de confirmation :
```rust
pub enum ConfirmAction {
    // ... existants ...
    ResetSoft(Oid),   // Reset soft vers ce commit
    ResetHard(Oid),   // Reset hard vers ce commit
}
```

### 9.3 — Ajouter le flux UI (priorité haute)

Le flux proposé en 2 étapes :

1. **Touche `R` dans la vue graph** → `GitAction::ResetPrompt`
2. **Le handler** affiche un petit picker/popup pour choisir le mode :
   - `s` : Soft reset (garde les modifications stagées)
   - `h` : Hard reset (perd toutes les modifications)
   - `Esc` : Annuler

   Option alternative plus simple : utiliser directement le dialogue de confirmation avec le choix intégré dans le message.

3. **Après le choix du mode**, créer `ConfirmAction::ResetSoft(oid)` ou `ConfirmAction::ResetHard(oid)`
4. **Confirmation** → exécuter `reset_to_commit()` + `mark_dirty()` + flash message

**`src/ui/input.rs`** — Ajouter dans les keybindings graph (vers ligne 290) :
```rust
KeyCode::Char('R') => Some(AppAction::Git(GitAction::ResetPrompt)),
```

**`src/handler/git.rs`** — Ajouter le handler :
```rust
fn handle_reset_prompt(state: &mut AppState) -> Result<()> {
    // Récupérer le commit sélectionné
    if let Some(row) = state.graph_view.rows.selected_item() {
        let oid = row.node.oid;
        let short_hash = &row.node.short_hash;
        // Ouvrir un popup de choix (ou directement un ConfirmAction)
        state.reset_picker = Some(ResetPickerState { target_oid: oid, ... });
    }
    Ok(())
}
```

**`src/handler/dispatcher.rs`** — Dans `handle_confirm_action()` :
```rust
ConfirmAction::ResetSoft(oid) => {
    ctx.state.repo.reset_to_commit(oid, ResetType::Soft)?;
    ctx.state.set_flash_message("Reset soft effectué".to_string());
    ctx.state.mark_dirty();
}
ConfirmAction::ResetHard(oid) => {
    ctx.state.repo.reset_to_commit(oid, ResetType::Hard)?;
    ctx.state.set_flash_message("Reset hard effectué".to_string());
    ctx.state.mark_dirty();
}
```

### 9.4 — UI du picker de reset (priorité haute)

Deux options d'implémentation :

**Option A — Popup simple (recommandée)** : Un petit overlay centré style le `confirm_dialog` existant :

```
┌─── Reset vers a1b2c3d ───┐
│                           │
│  s  Soft  (garde staged)  │
│  h  Hard  (perd tout)     │
│                           │
│  Esc pour annuler         │
└───────────────────────────┘
```

Créer `src/ui/reset_picker.rs` avec un `render()` simple, ou réutiliser le pattern du `merge_picker.rs`.

**Option B — Confirmation en 2 temps** : Le `R` ouvre directement le dialogue de confirmation pour un Soft reset, avec `Shift+R` pour Hard. Plus simple mais moins découvrable.

→ Recommandation : **Option A** pour la clarté.

### 9.5 — État et aide (priorité moyenne)

- Ajouter `reset_picker: Option<ResetPickerState>` dans `AppState`
- Ajouter `R` dans la barre d'aide de la vue graph (`src/ui/graph_view.rs` ou `help_overlay.rs`)
- Mettre à jour la doc des keybindings

### 9.6 — Sécurité (priorité haute)

Le reset hard est une opération destructive. S'assurer :

- Le dialogue de confirmation affiche clairement le hash et le message du commit cible
- Pour le hard reset, ajouter un avertissement : `⚠ Les modifications non committées seront perdues`
- Vérifier que `mark_dirty()` est bien appelé après pour rafraîchir le graph

---

## Critères de validation

- [ ] `R` ouvre le picker de reset dans la vue graph
- [ ] Soft reset fonctionne (HEAD déplacé, modifications stagées)
- [ ] Hard reset fonctionne (HEAD déplacé, working tree réinitialisé)
- [ ] Le dialogue de confirmation affiche un avertissement pour le hard reset
- [ ] Le graph se rafraîchit après le reset
- [ ] `cargo clippy` propre
- [ ] `cargo test` passe (ajouter des tests unitaires pour `reset_to_commit`)
