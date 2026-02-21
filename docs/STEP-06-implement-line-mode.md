# STEP-06 : Implémenter le vrai mode Ligne

## Problème

Le mode "Ligne" (`ConflictResolutionMode::Line`) existe visuellement (on peut sélectionner une ligne avec `line_selected`) mais la résolution se fait toujours au niveau de la section entière. Les `line_resolutions` ne sont jamais peuplées par les handlers `o`/`t`/`b`. Il faut permettre de choisir ligne par ligne quel côté garder.

## Prérequis

- STEP-04 (les touches de mode fonctionnent)
- STEP-03 (le scroll fonctionne, nécessaire pour naviguer dans les lignes)

## Fichiers concernés

| Fichier | Lignes | Modification |
|---------|--------|-------------|
| `src/git/conflict.rs` | `ConflictSection` struct, `LineResolution` | Vérifier la structure `line_resolutions` |
| `src/event.rs` | `handle_conflict_choose_ours/theirs/both()` (~2237-2293) | En mode Ligne, peupler `line_resolutions[line_selected]` au lieu de `section.resolution` |
| `src/event.rs` | Navigation ligne (~handlers ConflictLineUp/Down) | S'assurer que `line_selected` est borné correctement |
| `src/git/conflict.rs` | `generate_resolved_content_with_source()` (~955+) | Le code Line mode existe déjà, vérifier qu'il lit bien `line_resolutions` |
| `src/ui/conflicts_view.rs` | `render_ours_panel()`, `render_theirs_panel()` | Surligner la ligne sélectionnée, indiquer les lignes déjà résolues |

## Détail des modifications

### 1. `src/git/conflict.rs` — Initialisation des `line_resolutions`

Vérifier que lors du parsing (`parse_conflict_file()`), chaque `ConflictSection` initialise `line_resolutions` avec la bonne taille :

```rust
// Pour chaque section parsée :
section.line_resolutions = vec![
    LineResolution::Unresolved; 
    section.ours.len().max(section.theirs.len())
];
```

Ou mieux, deux vecteurs séparés (un pour ours, un pour theirs) puisque les lignes ne correspondent pas forcément 1:1.

**Architecture proposée** : Chaque ligne dans `ours` et `theirs` a un état `included: bool`. Initialement toutes les lignes ours sont incluses et toutes les lignes theirs exclues (ou vice versa, selon la résolution globale si elle existe).

```rust
pub struct LineLevelResolution {
    pub ours_lines_included: Vec<bool>,   // true = cette ligne ours est dans le résultat
    pub theirs_lines_included: Vec<bool>, // true = cette ligne theirs est dans le résultat
}
```

### 2. `src/event.rs` — Handler `Enter` en mode Ligne

Quand l'utilisateur est en mode Ligne sur le panneau Ours et appuie sur `Enter` :
- La ligne `line_selected` du côté courant (ours si panneau Ours) est **togglée** (incluse/exclue du résultat)
- Cela met à jour `line_resolutions` de la section courante

```rust
fn handle_conflict_enter_line_mode(&mut self) {
    if let Some(ref mut cs) = self.state.conflicts_state {
        if cs.resolution_mode != ConflictResolutionMode::Line {
            return;
        }
        
        let file = &mut cs.files[cs.selected_file];
        let section = &mut file.conflicts[cs.section_selected];
        
        match cs.focus {
            ConflictPanelFocus::OursPanel => {
                if let Some(lr) = &mut section.line_level_resolution {
                    let idx = cs.line_selected;
                    if idx < lr.ours_lines_included.len() {
                        lr.ours_lines_included[idx] = !lr.ours_lines_included[idx];
                    }
                }
            }
            ConflictPanelFocus::TheirsPanel => {
                if let Some(lr) = &mut section.line_level_resolution {
                    let idx = cs.line_selected;
                    if idx < lr.theirs_lines_included.len() {
                        lr.theirs_lines_included[idx] = !lr.theirs_lines_included[idx];
                    }
                }
            }
            _ => {}
        }
        
        // Vérifier si la section est résolue (au moins une ligne sélectionnée)
        section.check_line_resolution();
    }
}
```

### 3. `src/ui/conflicts_view.rs` — Rendu en mode Ligne

Dans les panneaux Ours/Theirs en mode Ligne :
- Chaque ligne affiche un indicateur : `[x]` si incluse, `[ ]` si exclue
- La ligne `line_selected` est surlignée (fond distinct)
- Les lignes déjà incluses dans le résultat ont une couleur verte/bleue

```rust
// Exemple de rendu d'une ligne en mode Ligne :
let indicator = if is_included { "[x] " } else { "[ ] " };
let style = if is_current_line {
    Style::default().bg(Color::DarkGray) // Ligne sélectionnée
} else if is_included {
    Style::default().fg(Color::Green) // Ligne incluse
} else {
    Style::default().fg(Color::DarkGray) // Ligne exclue
};
```

### 4. `src/git/conflict.rs` — Génération du contenu résolu en mode Ligne

`generate_resolved_content_with_source()` doit gérer le mode Ligne :

```rust
ConflictResolutionMode::Line => {
    if let Some(ref lr) = section.line_level_resolution {
        // Inclure les lignes ours marquées comme incluses
        for (i, line) in section.ours.iter().enumerate() {
            if lr.ours_lines_included.get(i) == Some(&true) {
                resolved_lines.push(ResolvedLine {
                    content: line.clone(),
                    source: LineSource::Ours,
                });
            }
        }
        // Puis les lignes theirs marquées comme incluses
        for (i, line) in section.theirs.iter().enumerate() {
            if lr.theirs_lines_included.get(i) == Some(&true) {
                resolved_lines.push(ResolvedLine {
                    content: line.clone(),
                    source: LineSource::Theirs,
                });
            }
        }
    }
}
```

### 5. Détection de résolution d'une section en mode Ligne

Une section est considérée "résolue" en mode Ligne quand l'utilisateur a explicitement validé ses choix (au moins une action effectuée). On peut ajouter un flag `line_resolution_touched: bool` pour suivre ça.

## Tests

- Passer en mode Ligne (`L`), naviguer dans le panneau Ours.
- Appuyer sur `Enter` pour toggler l'inclusion d'une ligne.
- Vérifier que le panneau Result reflète uniquement les lignes sélectionnées.
- Sélectionner des lignes des deux côtés et vérifier le résultat combiné.
- Vérifier que la section est marquée résolue après édition.
