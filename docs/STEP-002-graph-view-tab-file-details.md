# STEP-002 : Vue Graph — Tab doit naviguer vers les fichiers du commit, pas vers Status

## Problème

Dans la vue Graph, appuyer sur Tab affiche un panneau « Status (0 fichiers) » au lieu de
permettre de naviguer dans les fichiers du commit sélectionné et voir le diff correspondant.

**Cause racine (2 bugs liés) :**

1. **Tab est mappé à `SwitchBottomMode`** qui toggle entre `Files` (fichiers du commit)
   et `Parents` (working directory status), au lieu de changer le **focus** entre les panneaux
   (Graph → BottomLeft → BottomRight → Graph).

2. **`selected_file_diff` n'est jamais peuplé.** Le champ `state.selected_file_diff`
   (src/state/mod.rs, ligne ~82) est initialisé à `None` et aucun code ne le met à jour.
   Le diff panel en bas à droite montre donc toujours le placeholder, même quand le focus
   est sur les fichiers.

## Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/ui/input.rs` | Mapping de Tab → `SwitchBottomMode` (ligne ~268) |
| `src/handler/dispatcher.rs` | `SwitchBottomMode` toggle bottom_left_mode (lignes ~92-102) |
| `src/handler/navigation.rs` | `SwitchPanel` cycle de focus (lignes ~148-158) — jamais câblé à une touche |
| `src/state/mod.rs` | `selected_file_diff` (ligne ~82) — jamais écrit, `sync_legacy_selection()` (lignes ~224-236) |
| `src/handler/navigation.rs` | `handle_file_up/down` (lignes ~177-187) — ne charge pas le diff |
| `src/ui/mod.rs` | `render_graph_view()` (ligne ~109) — switching entre detail_view et diff_view selon focus |
| `src/ui/files_view.rs` | Affichage "Status (N fichiers)" vs "Fichiers" (lignes ~27-37) |
| `src/ui/diff_view.rs` | Rendu du diff (ou placeholder) en bas à droite |
| `src/ui/detail_view.rs` | Rendu des métadonnées commit en bas à droite |

## Plan de correction

### 1. Remapper Tab pour cycler le focus entre panneaux

Dans `src/ui/input.rs`, changer le mapping de Tab (ligne ~268) :

```rust
// AVANT
KeyCode::Tab => Some(AppAction::SwitchBottomMode),
// APRÈS
KeyCode::Tab => Some(AppAction::SwitchPanel),
```

Ajouter un autre raccourci (ex: `m`) pour `SwitchBottomMode` si la fonctionnalité reste souhaitée.

### 2. Charger le diff du fichier sélectionné

Créer une fonction `load_commit_file_diff(state)` qui peuple `state.selected_file_diff` :

```rust
pub fn load_commit_file_diff(state: &mut AppState) -> Result<()> {
    if let Some(row) = state.graph.get(state.selected_index) {
        if let Some(file) = state.commit_files.get(state.file_selected_index) {
            state.selected_file_diff = state.repo.file_diff(row.node.oid, &file.path).ok();
            state.graph_view.diff_scroll_offset = 0;
            return Ok(());
        }
    }
    state.selected_file_diff = None;
    Ok(())
}
```

### 3. Appeler `load_commit_file_diff` aux bons endroits

- **`AppAction::Select` (Enter sur un commit)** dans dispatcher.rs (lignes ~111-122) :
  après avoir chargé `commit_files`, appeler `load_commit_file_diff()`.

- **`handle_file_up()` / `handle_file_down()`** dans navigation.rs (lignes ~177-187) :
  après avoir changé `file_selected_index`, appeler `load_commit_file_diff()`.

- **`SwitchPanel`** : quand le focus passe à `BottomLeft`, charger le diff du premier fichier.

- **`sync_legacy_selection()`** dans state/mod.rs (lignes ~224-236) :
  après avoir chargé `commit_files`, appeler `load_commit_file_diff()`.

### 4. Sélectionner le premier fichier par défaut

Dans `AppAction::Select` et `sync_legacy_selection()`, s'assurer que `file_selected_index = 0`
et que le diff est chargé immédiatement pour ce premier fichier.

### 5. Vérification

- [ ] Tab dans la vue graph cycle : Graph → Fichiers (bottom-left) → Détail (bottom-right) → Graph
- [ ] Quand le focus est sur Fichiers, le panneau bottom-right montre le diff du fichier sélectionné
- [ ] Navigation j/k dans la liste de fichiers met à jour le diff en temps réel
- [ ] Enter sur un commit affiche les fichiers avec le premier sélectionné et son diff visible
- [ ] Quand le focus est sur Graph ou Détail, le panneau bottom-right montre les métadonnées du commit
