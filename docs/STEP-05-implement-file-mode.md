# STEP-05 : Implémenter le vrai mode Fichier

## Problème

Le mode "Fichier" (`ConflictResolutionMode::File`) est actuellement identique au mode "Bloc" dans le code. En mode Fichier, l'action `Enter` depuis le panneau Ours ou Theirs devrait résoudre **toutes les sections du fichier d'un coup** en faveur du côté sélectionné (pas section par section).

## Prérequis

- STEP-04 (les touches F/B/L fonctionnent correctement)

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/event.rs` | `handle_conflict_choose_ours()` (~2237), `handle_conflict_choose_theirs()` (~2260), `handle_conflict_choose_both()` (~2280) | Adapter le comportement selon le mode |
| `src/event.rs` | `handle_conflict_resolve_file()` (~2340) | En mode Fichier, `Enter` valide le choix pour toutes les sections |
| `src/ui/conflicts_view.rs` | `render_ours_panel()`, `render_theirs_panel()` | En mode Fichier, surligner tout le contenu (pas juste une section) |
| `src/git/conflict.rs` | `generate_resolved_content_with_source()` (~895) | Différencier le rendu File vs Block |

## Détail des modifications

### 1. `src/event.rs` — Comportement du mode Fichier

En mode **Fichier**, quand l'utilisateur est sur le panneau Ours et appuie sur `Enter` :
- **Toutes** les sections du fichier courant reçoivent `resolution = Some(ConflictResolution::Ours)`
- Le fichier est marqué comme résolu (`is_resolved = true`)
- On passe automatiquement au fichier suivant non résolu

```rust
fn handle_conflict_enter_file_mode(&mut self) {
    if let Some(ref mut cs) = self.state.conflicts_state {
        let resolution = match cs.focus {
            ConflictPanelFocus::OursPanel => ConflictResolution::Ours,
            ConflictPanelFocus::TheirsPanel => ConflictResolution::Theirs,
            _ => return, // Ne rien faire si on n'est pas sur Ours/Theirs
        };
        
        // Résoudre toutes les sections du fichier courant
        if let Some(file) = cs.files.get_mut(cs.selected_file) {
            for section in &mut file.conflicts {
                section.resolution = Some(resolution.clone());
            }
            file.is_resolved = true;
        }
    }
}
```

### 2. `src/ui/conflicts_view.rs` — Rendu visuel en mode Fichier

En mode Fichier, le panneau Ours/Theirs doit visuellement indiquer que c'est **tout le fichier** qui sera sélectionné :
- Ne pas mettre en surbrillance une section spécifique
- Optionnel : ajouter une indication "[Fichier entier]" dans le titre ou une bordure spéciale
- La navigation haut/bas ne change plus de section (pas de sens en mode Fichier)

### 3. Navigation en mode Fichier

En mode Fichier dans les panneaux Ours/Theirs :
- `Up`/`Down` → `ConflictNextFile` / `ConflictPrevFile` (naviguer entre fichiers, pas entre sections)
- Ou simplement désactiver la navigation section (scroll libre uniquement)

## Tests

- Passer en mode Fichier (`F`), se placer sur le panneau Ours, appuyer sur `Enter` : toutes les sections doivent être résolues en "Ours".
- Vérifier que le fichier est marqué résolu dans la liste.
- Passer au fichier suivant et répéter avec Theirs.
- Vérifier que le panneau Result affiche le contenu correct (tout ours ou tout theirs).
