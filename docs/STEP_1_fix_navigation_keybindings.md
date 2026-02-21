# STEP 1 - Correction de la navigation et des keybindings dans la vue Conflits

## Problème

1. **Les flèches haut/bas ne permettent pas de naviguer entre les fichiers** quand on est sur le panneau `FileList`. Elles sont mappées à `ConflictNextSection`/`ConflictPrevSection` (ou `ConflictLineDown`/`ConflictLineUp` en mode ligne), peu importe le panneau actif.
2. **Tab fait "fichier suivant"** au lieu de basculer entre les panneaux (`FileList` → `Ours` → `Theirs` → `Result`). Le switch panneau est censé être sur `Shift+Tab`, mais le binding `KeyCode::Char('\t') + SHIFT` n'est jamais déclenché (les terminaux envoient `KeyCode::BackTab`), et `BackTab` est déjà pris par `ConflictPrevFile`.

## Fichiers à modifier

| Fichier | Rôle |
|---------|------|
| `src/ui/input.rs` | `map_conflicts_key()` (ligne ~382) |
| `src/event.rs` | Handlers de navigation conflits (lignes ~2281-2490) |
| `src/state.rs` | `ConflictsState` si des champs supplémentaires sont nécessaires |
| `src/ui/conflicts_view.rs` | Help overlay et help bar pour refléter les nouveaux bindings |

## Modifications détaillées

### 1. `src/ui/input.rs` — `map_conflicts_key()`

Revoir complètement la logique de mapping pour que le comportement dépende du **panneau actif** (`panel_focus`).

Nouveau mapping :

```
Tab                → ConflictSwitchPanel       (toujours)
Shift+Tab (BackTab)→ ConflictSwitchPanelReverse (toujours, cycle inverse)

Flèches / j/k     → dépend du panneau actif :
  - FileList       → ConflictNextFile / ConflictPrevFile
  - OursPanel      → selon le mode :
      - File mode  → rien (un seul choix par fichier)
      - Block mode → ConflictNextSection / ConflictPrevSection
      - Line mode  → ConflictLineDown / ConflictLineUp
  - TheirsPanel    → idem OursPanel
  - ResultPanel    → ConflictResultScrollDown / ConflictResultScrollUp
```

Supprimer les anciens bindings :
- `Tab → ConflictNextFile` (supprimé)
- `BackTab → ConflictPrevFile` (supprimé)
- `Char('\t') + SHIFT → ConflictSwitchPanel` (supprimé, dead code)

Le `panel_focus` est déjà accessible via `state.conflicts_state.as_ref().map(|s| s.panel_focus)`.

### 2. `src/state.rs` — Nouvelles actions

Ajouter dans l'enum `AppAction` :

```rust
/// Switch vers le panneau suivant (Tab).
ConflictSwitchPanelForward,
/// Switch vers le panneau précédent (Shift+Tab).
ConflictSwitchPanelReverse,
/// Scroll vers le bas dans le panneau résultat.
ConflictResultScrollDown,
/// Scroll vers le haut dans le panneau résultat.
ConflictResultScrollUp,
```

Supprimer `ConflictSwitchPanel` (remplacé par Forward/Reverse).

### 3. `src/event.rs` — Handlers

- **Renommer** `handle_conflict_switch_panel` en `handle_conflict_switch_panel_forward` : cycle `FileList → OursPanel → TheirsPanel → ResultPanel → FileList`.
- **Ajouter** `handle_conflict_switch_panel_reverse` : cycle inverse `FileList → ResultPanel → TheirsPanel → OursPanel → FileList`.
- **Ajouter** `handle_conflict_result_scroll_down` et `handle_conflict_result_scroll_up` : incrémentent/décrémentent `result_scroll` dans `ConflictsState`.
- La navigation **flèches dans FileList** est déjà implémentée (`handle_conflict_next_file` / `handle_conflict_prev_file`), elle sera simplement appelée par le nouveau mapping.

### 4. `src/ui/conflicts_view.rs` — Help bar et help overlay

Mettre à jour les textes d'aide :

**Help bar** (ligne ~96) :
```
"Tab:panneau  ↑/↓:naviguer  o:ours  t:theirs  b:both  F:mode  Enter:valider  V:finaliser  q:abort"
```

**Help overlay** (ligne ~476) :
```
Navigation :
  ↑/↓ ou j/k  - Naviguer (fichiers / sections / lignes selon le panneau)
  Tab          - Panneau suivant (Fichiers → Ours → Theirs → Résultat)
  Shift+Tab    - Panneau précédent
```

## Résultat attendu

- `Tab` bascule entre les 4 panneaux dans l'ordre.
- `Shift+Tab` bascule en sens inverse.
- Les flèches naviguent dans le **contexte du panneau actif** :
  - `FileList` → fichier suivant/précédent
  - `OursPanel` / `TheirsPanel` → section ou ligne selon le mode
  - `ResultPanel` → scroll du contenu résultat
