# STEP-02 : Restaurer la vue détail commit (fichiers modifiés et diff)

## Problème

Depuis la vue Graph, l'utilisateur ne peut plus voir les détails d'un commit (fichiers modifiés et contenu du diff). Le panneau bas-droit affiche uniquement les métadonnées du commit (hash, auteur, date, message) quand le focus est sur `Graph`, `Detail`, ou `BottomRight`.

Pour voir le diff d'un fichier, il faut :
1. Appuyer sur `Tab` pour basculer le focus vers `BottomLeft` (panneau fichiers)
2. Naviguer dans les fichiers avec `j`/`k`
3. Le panneau bas-droit bascule alors automatiquement sur le diff

**Le problème** : cette navigation n'est pas intuitive. `Enter` sur un commit dans le graphe ne fait rien (`AppAction::Select` est un no-op dans `dispatcher.rs:97`). L'utilisateur s'attend à ce que `Enter` ouvre les détails du commit ou que le panneau bas-droit montre automatiquement les fichiers/diff.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/ui/mod.rs` | 143-168 | Logique de rendu contextuel du panneau bas-droit |
| `src/handler/dispatcher.rs` | 95-98 | `AppAction::Select` = no-op |
| `src/ui/input.rs` | 200 | `Enter` → `AppAction::Select` |
| `src/handler/navigation.rs` | 165-184 | `handle_switch_panel` — cycle `Graph → BottomLeft → BottomRight` |
| `src/ui/layout.rs` | 1-92 | Layout : `bottom_left` (50%) + `bottom_right` (50%) |
| `src/ui/diff_view.rs` | - | Rendu du diff (fonctionne, mais non visible par défaut) |

## Analyse du flux actuel

```
Focus: Graph     → bas-droit = detail_view (métadonnées commit)
Focus: BottomLeft → bas-droit = diff_view (diff du fichier sélectionné)
Focus: BottomRight → bas-droit = detail_view (métadonnées commit)
```

Le diff n'est visible que quand le focus est sur `BottomLeft` (`Files`). C'est le bloc dans `src/ui/mod.rs:143-168` :

```rust
match state.focus {
    FocusPanel::Graph | FocusPanel::Detail | FocusPanel::BottomRight => {
        detail_view::render(/* ... */);  // Métadonnées seulement
    }
    FocusPanel::Files | FocusPanel::BottomLeft => {
        diff_view::render(/* ... */);  // Diff du fichier
    }
}
```

## Solution proposée

### Option A : `Enter` bascule le focus vers le panneau fichiers (recommandée)

1. **Modifier `src/handler/dispatcher.rs`** — `AppAction::Select` en mode Graph :
   ```rust
   AppAction::Select => {
       if ctx.state.view_mode == ViewMode::Graph
           && ctx.state.focus == FocusPanel::Graph
       {
           ctx.state.focus = FocusPanel::BottomLeft;
           // Recharger les fichiers du commit sélectionné
           ctx.state.file_selected_index = 0;
       }
       Ok(())
   }
   ```

2. Cela suffit car le rendu dans `src/ui/mod.rs` basculera automatiquement vers `diff_view` quand le focus passe à `BottomLeft`.

3. **Ajouter `Esc` pour revenir** — déjà implémenté dans `input.rs:159-163` :
   ```rust
   if key.code == KeyCode::Esc {
       if state.focus == FocusPanel::Detail {
           return Some(AppAction::SwitchBottomMode);
       }
   }
   ```
   Étendre pour aussi revenir de `BottomLeft` vers `Graph`.

### Option B : Afficher les fichiers + diff sans changer de focus

1. **Modifier `src/ui/mod.rs`** — toujours afficher le diff dans le panneau bas-droit (même quand le focus est sur `Graph`) si des fichiers de commit sont chargés.

2. Le panneau bas-gauche affiche les fichiers, le panneau bas-droit le diff, indépendamment du focus.

## Ordre d'implémentation

1. Modifier `AppAction::Select` dans `dispatcher.rs` pour basculer vers `BottomLeft`
2. S'assurer que `Esc` depuis `BottomLeft` retourne au focus `Graph`
3. Vérifier que `state.commit_files` est bien chargé lors du changement de commit
4. Tester le flux : sélectionner un commit → Enter → voir les fichiers → j/k → voir le diff → Esc → retour au graphe

## Critère de validation

- `Enter` sur un commit dans le graphe affiche la liste des fichiers modifiés et le diff
- `Esc` permet de revenir au graphe
- La navigation dans les fichiers met à jour le diff en temps réel
