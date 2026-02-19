# STEP-009 — Feature : Sélection de texte à la souris

## Problème

Il n'est pas possible de sélectionner du texte à la souris dans l'application. Quand on essaie, rien ne se passe car crossterm capture les événements souris et l'application ne gère que le scroll (voir `map_mouse()` dans `input.rs` lignes 375-426).

## Fichiers concernés

- `src/ui/input.rs` — `map_mouse()` (l375-426) : gestion des événements souris
- `src/state.rs` — Structures d'état (ajouter état de sélection)
- `src/event.rs` — Event loop et gestion des événements
- `src/terminal.rs` — Configuration du terminal crossterm
- `src/ui/diff_view.rs` — Rendu du diff (pour afficher la sélection)
- `src/ui/graph_view.rs` — Rendu du graph (pour afficher la sélection)

## Analyse

### Contrainte TUI

Dans un terminal, la sélection de texte native est gérée par l'émulateur de terminal. Quand crossterm active le mode "mouse capture" (`EnableMouseCapture`), les événements souris sont envoyés à l'application au lieu du terminal, ce qui **désactive** la sélection native.

### Deux approches possibles

#### Option A — Désactiver la capture souris (simple)

Désactiver `EnableMouseCapture` dans la config crossterm. La sélection native du terminal fonctionne automatiquement. On perd le scroll souris et les clics.

```rust
// Dans terminal.rs
crossterm::terminal::enable_raw_mode()?;
execute!(stdout, EnterAlternateScreen)?; // PAS de EnableMouseCapture
```

#### Option B — Implémenter la sélection dans l'application (complexe)

Capturer les événements `MouseDown` + `MouseDrag` + `MouseUp` pour tracer une sélection, puis copier le texte sélectionné dans le clipboard.

```rust
/// État de la sélection souris.
pub struct MouseSelection {
    /// Position de début (colonne, ligne).
    pub start: (u16, u16),
    /// Position de fin (colonne, ligne).
    pub end: (u16, u16),
    /// Sélection en cours (dragging).
    pub is_selecting: bool,
    /// Texte sélectionné.
    pub selected_text: Option<String>,
}
```

##### Événements à gérer :

```rust
fn map_mouse(mouse: MouseEvent, state: &AppState) -> Option<AppAction> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Démarrer la sélection
            Some(AppAction::StartSelection(mouse.column, mouse.row))
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            // Étendre la sélection
            Some(AppAction::ExtendSelection(mouse.column, mouse.row))
        }
        MouseEventKind::Up(MouseButton::Left) => {
            // Finaliser la sélection, copier dans le clipboard
            Some(AppAction::EndSelection(mouse.column, mouse.row))
        }
        // ...
    }
}
```

##### Rendu de la sélection :

Surliguer les cellules sélectionnées avec un style `bg(Color::Blue)` lors du rendu.

##### Clipboard :

Utiliser la crate `arboard` ou `clipboard` pour copier dans le clipboard système :

```rust
// Cargo.toml
arboard = "3"

// Dans event.rs
fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
```

#### Option C — Mode hybride (recommandé)

Garder la capture souris mais permettre de la désactiver temporairement :

- Par défaut : capture souris activée (scroll + clics)
- Maintenir `Shift` + clic : Laisser passer la sélection au terminal (la plupart des émulateurs de terminal passent les événements avec Shift au terminal même en mode capture)
- Ou touche `y` pour copier le contenu visible d'un panneau dans le clipboard

### Recommandation

**Option C** est le meilleur compromis. La plupart des TUI (lazygit, etc.) fonctionnent ainsi :
- Le scroll souris est conservé
- `Shift+clic` permet la sélection native du terminal
- La plupart des émulateurs (iTerm2, Alacritty, kitty, WezTerm) supportent déjà le bypass Shift

En complément, ajouter une touche `y` ("yank") pour copier le contenu du panneau actif.

## Solution retenue (Option C)

### Étape 1 — Documenter le comportement Shift+clic

Ajouter dans l'aide (`?`) une mention que `Shift+clic` permet la sélection native.

### Étape 2 — Ajouter la touche `y` (yank/copie)

```rust
// Nouvelle action
AppAction::CopyPanelContent,

// Dans map_key(), contexte Graph/Staging
KeyCode::Char('y') => Some(AppAction::CopyPanelContent),
```

Le handler copie le contenu du panneau focalisé :
- Graph : hash + message du commit sélectionné
- Files : chemin du fichier sélectionné
- Detail/Diff : contenu du diff affiché
- Staging : diff du fichier sélectionné

### Étape 3 — Dépendance clipboard

Ajouter `arboard` dans `Cargo.toml` et implémenter le handler.

## Tests

- Vérifier que le scroll souris fonctionne toujours
- Vérifier que `Shift+clic` permet la sélection dans iTerm2 / Terminal.app
- Vérifier que `y` copie le contenu approprié selon le panneau
- Vérifier le flash message "Copié dans le clipboard"
