# STEP-003 — Bug : Vue Graph - Sélection fichier automatique + thème clair

## Problème (2 sous-problèmes)

### 3a — Pas de fichier sélectionné par défaut

Quand on navigue vers le panneau "Files" (Tab) en vue Graph, aucun fichier n'est pré-sélectionné. L'utilisateur voit le message "Sélectionnez un fichier pour voir le diff" et doit appuyer sur j/k pour commencer à naviguer. Le premier fichier devrait etre automatiquement sélectionné.

### 3b — Code non modifié invisible en thème clair

Dans le diff affiché en vue Graph (panneau Detail), les lignes de code non modifiées (contexte) sont affichées en blanc (`Color::White`), ce qui les rend invisibles sur un fond clair.

## Fichiers concernés

- `src/event.rs` — Logique de switch de focus vers Files
- `src/ui/files_view.rs` — Rendu de la liste de fichiers
- `src/ui/diff_view.rs` — Rendu du diff (couleurs hardcodées)
- `src/ui/theme.rs` — Système de thème (non utilisé par la plupart des vues)

## Solution 3a — Sélection automatique du premier fichier

Quand le focus passe à `FocusPanel::Files`, si `file_selected_index` est 0 et qu'il y a des fichiers disponibles, charger automatiquement le diff du premier fichier :

```rust
// Dans event.rs, lors du SwitchBottomMode vers Files
AppAction::SwitchBottomMode => {
    match state.focus {
        FocusPanel::Graph => {
            state.focus = FocusPanel::Files;
            // Auto-sélectionner le premier fichier s'il y en a
            if !state.commit_files.is_empty() && state.selected_file_diff.is_none() {
                state.file_selected_index = 0;
                // Charger le diff du premier fichier
                load_file_diff(state, 0);
            }
        }
        // ...
    }
}
```

## Solution 3b — Utiliser le thème pour les couleurs du diff

Remplacer les couleurs hardcodées dans `diff_view.rs` par les couleurs du thème. Deux approches possibles :

### Option A — Utiliser `Color::Reset` pour le texte de contexte

```rust
// Au lieu de Color::White pour le texte non modifié
let context_style = Style::default().fg(Color::Reset);
```

Cela utilise la couleur par défaut du terminal, qui sera noire sur fond clair et blanche sur fond sombre.

### Option B — Brancher le système de thème

Modifier `diff_view.rs` pour utiliser `THEME` :

```rust
use crate::ui::theme::THEME;

let context_color = THEME.text;  // Adapté au thème actif
let added_color = Color::Green;
let removed_color = Color::Red;
```

Et s'assurer que `theme.rs` définit des couleurs appropriées pour le thème clair :

```rust
pub fn light() -> Self {
    Self {
        text: Color::Black,           // Noir sur fond clair
        background: Color::White,
        // ...
    }
}
```

### Recommandation

Option A en quick fix (minimal), puis option B pour une refonte globale du thème (travail plus large qui pourrait etre un STEP séparé).

## Tests

- Naviguer vers Files via Tab et vérifier que le premier fichier est sélectionné et son diff affiché
- Passer en thème clair et vérifier que le texte de contexte dans le diff est lisible
- Vérifier que le thème sombre n'est pas cassé
