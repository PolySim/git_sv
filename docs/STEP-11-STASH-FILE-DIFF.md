# STEP 11 — Voir le diff des fichiers dans les stash

**Priorité** : Haute
**Effort estimé** : Moyen
**Impact** : Fonctionnalité attendue — on voit les fichiers mais pas leur contenu

---

## Constat

Quand on regarde un stash dans la vue Branches (onglet Stashes), on voit :
- Le message du stash
- L'index `stash@{N}`
- La branche d'origine
- La **liste des fichiers modifiés** avec leur statut (A/M/D/R)

Mais on ne peut pas voir **le contenu des modifications** d'un fichier. Le code d'infrastructure existe partiellement :

- **`stash_file_diff()`** dans `src/git/stash.rs` (ligne 136) : Génère le diff d'un fichier dans un stash — **implémenté et fonctionnel**
- **`stash_file_selected: usize`** dans `BranchesViewState` (ligne 47) : Index du fichier sélectionné — **existe dans l'état**
- **`stash_file_diff: Option<Vec<String>>`** dans `BranchesViewState` (ligne 48) : Cache du diff — **existe dans l'état**
- **Rendu du diff** dans `branches_view.rs` (lignes 495-525) : Affiche jusqu'à 30 lignes du diff si `stash_file_diff` est `Some` — **implémenté**

**Le chaînon manquant** : La **navigation entre les fichiers** (`h`/`l` selon la barre d'aide) n'est pas câblée. Aucun handler ne met à jour `stash_file_selected` ni ne charge `stash_file_diff`. La barre d'aide affiche `h/l:fichiers` mais les touches ne font rien.

### Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/git/stash.rs` | `stash_file_diff()` (ligne 136) — diff d'un fichier dans un stash |
| `src/git/repo.rs` | `stash_file_diff()` wrapper (ligne 135) |
| `src/ui/branches_view.rs` | `render_stash_detail()` (ligne 442) — rendu détail + mini diff (limité à 30 lignes) |
| `src/state/view/branches.rs` | `stash_file_selected`, `stash_file_diff` (lignes 47-48) |
| `src/state/action/branch.rs` | `BranchAction` — pas de variante pour naviguer les fichiers stash |
| `src/handler/branch.rs` | Handlers des actions branch/stash |
| `src/handler/navigation.rs` | Navigation stash (lignes 297-305) — seulement haut/bas dans la liste des stash |
| `src/ui/input.rs` | Keybindings de la vue branches |

---

## Actions à mener

### 11.1 — Câbler la navigation entre fichiers d'un stash (priorité haute)

C'est le plus critique — l'infrastructure existe, il faut juste connecter les pièces.

**`src/state/action/branch.rs`** — Ajouter des variantes :
```rust
pub enum BranchAction {
    // ... existants ...
    StashFileNext,  // Fichier suivant dans le stash
    StashFilePrev,  // Fichier précédent dans le stash
}
```

**`src/ui/input.rs`** — Dans les keybindings de la vue Branches, section Stashes :
```rust
// Quand on est sur l'onglet Stashes
KeyCode::Char('l') | KeyCode::Right => Some(AppAction::Branch(BranchAction::StashFileNext)),
KeyCode::Char('h') | KeyCode::Left => Some(AppAction::Branch(BranchAction::StashFilePrev)),
```

**`src/handler/branch.rs`** — Ajouter les handlers :
```rust
fn handle_stash_file_next(state: &mut AppState) -> Result<()> {
    if let Some(stash) = state.branches_view_state.stashes.selected_item() {
        let file_count = stash.files.len();
        if file_count > 0 {
            let idx = &mut state.branches_view_state.stash_file_selected;
            *idx = (*idx + 1).min(file_count - 1);
            // Charger le diff du fichier sélectionné
            load_stash_file_diff(state)?;
        }
    }
    Ok(())
}

fn handle_stash_file_prev(state: &mut AppState) -> Result<()> {
    let idx = &mut state.branches_view_state.stash_file_selected;
    *idx = idx.saturating_sub(1);
    load_stash_file_diff(state)?;
    Ok(())
}

fn load_stash_file_diff(state: &mut AppState) -> Result<()> {
    if let Some(stash) = state.branches_view_state.stashes.selected_item() {
        let idx = state.branches_view_state.stash_file_selected;
        if let Some(file) = stash.files.get(idx) {
            let diff = state.repo.stash_file_diff(stash.oid, &file.path)?;
            state.branches_view_state.stash_file_diff = Some(diff);
        }
    }
    Ok(())
}
```

### 11.2 — Charger automatiquement le diff du premier fichier (priorité haute)

Quand l'utilisateur sélectionne un stash (navigation haut/bas dans la liste), charger automatiquement le diff du premier fichier :

**`src/handler/navigation.rs`** — Après le changement de sélection dans la liste stash (lignes 297-305), ajouter :
```rust
// Réinitialiser la sélection fichier et charger le diff
state.branches_view_state.stash_file_selected = 0;
load_stash_file_diff(state)?;
```

### 11.3 — Améliorer le rendu du diff stash (priorité haute)

Le rendu actuel dans `render_stash_detail()` (lignes 495-525) est limité à 30 lignes. Améliorer :

- **Supprimer la limite de 30 lignes** : Utiliser un `Paragraph` avec scroll au lieu de tronquer
- **Ajouter le scroll** : Réutiliser le même pattern que le diff_view principal
  - Ajouter `stash_diff_scroll: usize` dans `BranchesViewState`
  - Touches `j`/`k` ou `Ctrl+d`/`Ctrl+u` pour scroller le diff quand le focus est sur le détail
- **Meilleure colorisation** : Aligner le rendu sur celui de `diff_view.rs` (numéros de ligne, headers de hunk en cyan, etc.)
- **Indicateur du fichier sélectionné** : Mettre en surbrillance le fichier actuellement affiché dans la liste

### 11.4 — Mode diff plein écran pour les stash (priorité moyenne)

Réutiliser le concept de STEP-08 (mode plein écran du diff) pour les stash :

- Quand un fichier de stash est sélectionné, `Enter` ou `z` ouvre le diff en plein écran
- Réutiliser `diff_view.rs` pour le rendu (au lieu du mini-rendu inline de `branches_view.rs`)
- Convertir les `Vec<String>` de `stash_file_diff` en `FileDiff` pour réutiliser le rendu existant

### 11.5 — Sécuriser le changement de stash (priorité basse)

Quand l'utilisateur change de stash sélectionné :
- Réinitialiser `stash_file_selected` à 0
- Réinitialiser `stash_file_diff` à `None`
- Réinitialiser le scroll du diff à 0

---

## Bugs existants découverts

### Bug 1 : StashDrop ne s'exécute pas après confirmation
Dans `src/handler/dispatcher.rs`, `handle_confirm_action()`, le cas `ConfirmAction::StashDrop(index)` tombe dans le catch-all `_ => { ctx.state.pending_confirmation = None; }` (vers ligne 442). Il faut ajouter un bras explicite :
```rust
ConfirmAction::StashDrop(index) => {
    crate::git::stash::drop_stash(&ctx.state.repo.repo, index)?;
    ctx.state.set_flash_message(format!("Stash @{{{}}} supprimé", index));
    ctx.state.mark_dirty();
}
```

### Bug 2 : Navigation fichiers stash non câblée
Comme décrit ci-dessus — `h`/`l` sont dans la barre d'aide mais ne font rien.

---

## Critères de validation

- [ ] `h`/`l` (ou `←`/`→`) naviguent entre les fichiers d'un stash
- [ ] Le diff du fichier sélectionné s'affiche automatiquement
- [ ] Le diff se charge aussi quand on change de stash
- [ ] Le diff n'est plus tronqué à 30 lignes (scroll possible)
- [ ] Le bug StashDrop est corrigé
- [ ] `cargo clippy` propre
- [ ] `cargo test` passe
