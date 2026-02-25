# STEP-004 : Vue Branch — Escape, saisie texte et confirmation dans les inputs (new/rename)

## Problème

Quand on appuie sur `n` (nouvelle branche) ou `r` (renommer), un champ de saisie apparaît
mais :
1. **On ne peut pas taper de texte** — les caractères ne s'affichent pas
2. **Escape ne ferme pas l'input** — on est bloqué dans le mode saisie
3. **Enter ne confirme pas l'action** — rien ne se passe

**Causes racines (3 bugs distincts) :**

### Bug A : Saisie texte inopérante
`AppAction::InsertChar(c)` est routé vers `EditHandler` (dispatcher.rs, ligne ~258) qui
modifie `staging_state.commit_message` au lieu de `branches_view_state.input_text`.
Même problème pour `DeleteChar`, `CursorLeft`, `CursorRight`.

### Bug B : Escape ne fait rien
`BranchAction::CancelInput` dans le match du handler (branch.rs, ligne ~36) retourne
`Ok(())` (no-op). La vraie fonction `handle_cancel_input()` (ligne ~313) existe mais
n'est jamais appelée — c'est du code mort.

### Bug C : Enter ne confirme pas
`BranchAction::ConfirmInput` dans le match du handler (branch.rs, ligne ~35) retourne
`Ok(())` (no-op avec commentaire "Géré par le handler d'édition"). La vraie fonction
`handle_confirm_input()` (ligne ~246) existe mais n'est jamais appelée — code mort aussi.

## Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/handler/branch.rs` | Match arms `ConfirmInput`/`CancelInput` (lignes ~35-36) — no-ops. Fonctions mortes `handle_confirm_input()` (ligne ~246), `handle_cancel_input()` (ligne ~313) |
| `src/handler/dispatcher.rs` | Routing `InsertChar`/`DeleteChar` → `EditHandler` (ligne ~258) au lieu de `BranchHandler` |
| `src/handler/edit.rs` | `EditHandler` — ne modifie que `staging_state.commit_message` (lignes ~32-39) |
| `src/ui/input.rs` | `map_branches_key()` en mode Input (lignes ~277-285) — mapping correct mais dispatch incorrect |
| `src/state/view/branches.rs` | `BranchesViewState` — champs `input_text`, `input_cursor`, `input_action` |
| `src/ui/branches_view.rs` | `render_input_overlay()` (ligne ~566) — affiche le champ de saisie |
| `src/git/branch.rs` | `create_branch()` (ligne ~164), `rename_branch()` (ligne ~184) |

## Plan de correction

### 1. Router les actions d'édition selon le contexte

Dans `src/handler/dispatcher.rs`, modifier le dispatch de `InsertChar`/`DeleteChar`/
`CursorLeft`/`CursorRight` pour vérifier si on est en mode input branches :

```rust
AppAction::InsertChar(c) => {
    if ctx.state.view_mode == ViewMode::Branches
        && ctx.state.branches_view_state.focus == BranchesFocus::Input
    {
        // Modifier le texte de l'input branches
        let pos = ctx.state.branches_view_state.input_cursor;
        ctx.state.branches_view_state.input_text.insert(pos, c);
        ctx.state.branches_view_state.input_cursor += 1;
    } else {
        // Comportement existant pour le commit message
        self.edit.handle(&mut ctx, EditAction::InsertChar(c))?;
    }
    Ok(())
}
```

Faire pareil pour `DeleteChar`, `CursorLeft`, `CursorRight`.

### 2. Câbler `CancelInput` à la vraie fonction

Dans `src/handler/branch.rs`, modifier le match arm :

```rust
// AVANT
BranchAction::CancelInput => Ok(()),
// APRÈS
BranchAction::CancelInput => Self::handle_cancel_input(state),
```

Où `handle_cancel_input` (déjà existante, ligne ~313) :
- Remet `focus` à `BranchesFocus::List`
- Clear `input_action`
- Clear `input_text`
- Reset `input_cursor`

### 3. Câbler `ConfirmInput` à la vraie fonction

Dans `src/handler/branch.rs`, modifier le match arm :

```rust
// AVANT
BranchAction::ConfirmInput => Ok(()),
// APRÈS
BranchAction::ConfirmInput => Self::handle_confirm_input(state),
```

Où `handle_confirm_input` (déjà existante, ligne ~246) gère :
- `InputAction::CreateBranch` → appelle `create_branch()` + checkout optionnel
- `InputAction::RenameBranch` → appelle `rename_branch()`
- Reset du focus, de l'input et refresh

### 4. Vérification

- [ ] Appuyer sur `n` → le champ de saisie s'affiche
- [ ] Taper un nom de branche → les caractères apparaissent
- [ ] Backspace supprime un caractère
- [ ] Flèches gauche/droite déplacent le curseur
- [ ] Enter confirme → la branche est créée / renommée
- [ ] Escape annule → retour à la liste sans créer de branche
- [ ] `r` pré-remplit le nom de la branche sélectionnée → on peut le modifier et confirmer
- [ ] Le commit message dans la vue staging continue de fonctionner normalement
