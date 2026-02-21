# STEP-08 : Choisir un fichier entier depuis la liste des fichiers

## Problème

Depuis le panneau FileList (liste des fichiers en conflit), l'utilisateur veut pouvoir rapidement choisir de garder un fichier entier (version ours ou theirs) sans devoir entrer dans chaque section.

## Prérequis

- STEP-05 (le mode Fichier fonctionne déjà pour résoudre toutes les sections)

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/ui/input.rs` | `map_conflicts_key()` | Ajouter des raccourcis depuis FileList |
| `src/event.rs` | Nouveau handler | Résoudre le fichier sélectionné en ours ou theirs |
| `src/ui/conflicts_view.rs` | `render_files_panel()` (~112-163) | Afficher les raccourcis et l'état visuel |
| `src/state.rs` | `AppAction` | Ajouter les nouvelles actions si nécessaire |

## Détail des modifications

### 1. `src/ui/input.rs` — Raccourcis depuis FileList

Quand le focus est sur le panneau FileList :

```rust
// Dans map_conflicts_key(), section FileList :
KeyCode::Char('o') | KeyCode::Left => Some(AppAction::ConflictFileChooseOurs),
KeyCode::Char('t') | KeyCode::Right => Some(AppAction::ConflictFileChooseTheirs),
```

**Choix UX** : `o` pour garder la version ours (branche courante), `t` pour theirs (branche mergée). Les flèches gauche/droite sont aussi intuitives (gauche = ours qui est à gauche dans le layout, droite = theirs).

### 2. `src/event.rs` — Handler de résolution rapide

```rust
fn handle_conflict_file_choose_ours(&mut self) {
    if let Some(ref mut cs) = self.state.conflicts_state {
        if let Some(file) = cs.files.get_mut(cs.selected_file) {
            // Résoudre toutes les sections en Ours
            for section in &mut file.conflicts {
                section.resolution = Some(ConflictResolution::Ours);
                // Réinitialiser les éventuelles résolutions par ligne
                section.line_level_resolution = None;
            }
            file.is_resolved = true;
            
            // Avancer au fichier suivant non résolu
            cs.advance_to_next_unresolved();
        }
    }
}

fn handle_conflict_file_choose_theirs(&mut self) {
    // Même logique avec ConflictResolution::Theirs
}
```

### 3. `src/ui/conflicts_view.rs` — Affichage dans FileList

Ajouter les raccourcis dans l'affichage du panneau FileList :

```rust
// En bas du panneau FileList ou dans la barre d'aide :
// "o/←:Garder ours  t/→:Garder theirs"
```

Améliorer l'affichage de chaque fichier dans la liste pour montrer l'état de résolution :

```
 ✓ src/main.rs          [Ours]
 ✓ src/lib.rs           [Theirs]
 ✗ src/utils.rs         [Non résolu]
 → src/config.rs        [Sélectionné]
```

### 4. Auto-résolution pour les cas simples

Pour les types de conflit spéciaux (`DeletedByUs`, `DeletedByThem`, `BothAdded`), la résolution depuis FileList est encore plus pertinente car il n'y a pas de sections à comparer ligne par ligne.

## Tests

- Sélectionner un fichier dans la liste, appuyer sur `o` : toutes ses sections passent en Ours, le fichier est marqué résolu.
- Sélectionner un fichier, appuyer sur `t` : idem en Theirs.
- Vérifier que le curseur avance au fichier suivant non résolu.
- Vérifier que le panneau Result reflète la résolution choisie.
- Tester avec un fichier `DeletedByThem` : `o` garde le fichier, `t` le supprime.
