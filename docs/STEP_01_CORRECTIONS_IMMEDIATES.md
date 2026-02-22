# STEP 01 - Corrections Immédiates

**Priorité**: 🔴 Haute  
**Effort estimé**: 1-2 heures  
**Risque**: Faible (corrections localisées)

---

## Objectif

Corriger tous les warnings Clippy, les imports inutilisés, les variables non utilisées et les bugs potentiels identifiés. Cette étape ne modifie pas l'architecture mais assainit la base de code.

---

## 1. Imports inutilisés à supprimer

### `src/app.rs:58-59`
```rust
// SUPPRIMER ces imports non utilisés:
use crate::state::{
    AppAction,        // ❌ Non utilisé
    AppState,         // ❌ Non utilisé
    FocusPanel,       // ❌ Non utilisé
    ViewMode,         // ❌ Non utilisé
    BottomLeftMode,
    BranchesFocus,
    BranchesSection,
    BranchesViewState,
    InputAction,
    StagingFocus,
    StagingState,
};
```

### `src/ui/confirm_dialog.rs:7`
```rust
// SUPPRIMER Wrap de l'import:
use ratatui::{
    widgets::{Block, Borders, Clear, Paragraph},  // Wrap supprimé
};
```

---

## 2. Variables inutilisées à corriger

### `src/event.rs:59`
```rust
// AVANT:
let had_flash = self.state.flash_message.is_some();

// APRÈS: Préfixer avec underscore ou supprimer si vraiment inutile
let _had_flash = self.state.flash_message.is_some();
```

### `src/event.rs:3229`
```rust
// AVANT: Variable assignée mais jamais lue
let mut text_to_copy = String::new();

// APRÈS: Analyser la logique - soit supprimer, soit utiliser correctement
// Cette variable semble être réassignée immédiatement après dans un match
```

### `src/git/commit.rs:70`
```rust
// AVANT:
let sig = repo.signature()?;

// APRÈS:
let _sig = repo.signature()?;
// OU supprimer si vraiment inutile
```

### `src/ui/nav_bar.rs:38`
```rust
// AVANT:
for (i, (key, label, mode)) in tabs.iter().enumerate() {

// APRÈS:
for (_i, (key, label, mode)) in tabs.iter().enumerate() {
// OU utiliser .iter() sans enumerate() si l'index n'est pas nécessaire
```

### `src/ui/status_bar.rs:15`
```rust
// AVANT:
fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    current_branch: Option<&str>,
    repo_path: &str,  // ❌ Non utilisé
    ...
)

// APRÈS: Soit utiliser, soit préfixer avec underscore
fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    current_branch: Option<&str>,
    _repo_path: &str,  // ou le supprimer du signature si vraiment inutile
    ...
)
```

---

## 3. Code mort à supprimer

### `src/error.rs:13` - Variant `Terminal` jamais utilisé
```rust
pub enum GitSvError {
    #[error("Erreur git : {0}")]
    Git(#[from] git2::Error),

    #[error("Erreur I/O : {0}")]
    Io(#[from] std::io::Error),

    // ❌ À SUPPRIMER - jamais construit
    // #[error("Erreur terminal : {0}")]
    // Terminal(String),

    #[error("Erreur clipboard : {0}")]
    Clipboard(String),
}
```

### `src/git/blame.rs:16-18` - Champs jamais lus
```rust
pub struct BlameLine {
    pub line_no: usize,
    pub content: String,
    pub commit_id: git2::Oid,
    pub author: String,
    // ❌ Ces champs ne sont jamais lus - soit les utiliser soit les supprimer
    pub author_email: String,  // Non utilisé
    pub timestamp: i64,        // Non utilisé
}
```

**Note**: Avant de supprimer ces champs, vérifier s'ils pourraient être utiles dans `blame_view.rs` pour afficher plus d'informations.

### `src/git/blame.rs:27` - Champ `path` jamais lu
```rust
pub struct FileBlame {
    pub path: String,  // ❌ Non utilisé - à supprimer ou utiliser
    pub lines: Vec<BlameLine>,
}
```

### `src/git/branch.rs:10,14` - Champs jamais lus
```rust
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,        // ❌ Non utilisé
    pub upstream: Option<String>,
    pub last_commit_date: Option<i64>,  // ❌ Non utilisé
}
```

**Note**: Ces champs pourraient être utiles pour l'affichage dans `branches_view.rs`. Considérer leur utilisation plutôt que leur suppression.

---

## 4. Bugs potentiels à corriger

### Bug 1: Troncature de chaîne non-safe avec UTF-8

#### `src/ui/graph_view.rs:97`
```rust
// AVANT: Peut paniquer si hash < 7 caractères
let short_hash = &hash[..7];

// APRÈS: Version safe
let short_hash = if hash.len() >= 7 { &hash[..7] } else { &hash };
```

#### `src/ui/blame_view.rs:95`
```rust
// AVANT: Peut paniquer sur caractères multi-octets UTF-8
let truncated = &blame_line.author[..author_width - 1];

// APRÈS: Version safe avec Unicode
let truncated: String = blame_line.author
    .chars()
    .take(author_width.saturating_sub(1))
    .collect();
```

### Bug 2: Valeur de hauteur de panel hardcodée

#### `src/event.rs` - Lignes 2596, 2621, 2946, 2965, 3007
```rust
// AVANT: Valeur hardcodée qui ne correspond pas à la réalité
let panel_height = 20usize;

// APRÈS: Idéalement, passer cette valeur depuis le contexte de rendu
// Pour l'instant, documenter ce TODO:
// TODO: La hauteur du panel devrait être passée depuis le contexte de rendu
let panel_height = 20usize;
```

### Bug 3: Index potentiellement invalide après modification du graph

#### `src/event.rs:1804-1836`
```rust
fn handle_next_search_result(&mut self) {
    if !self.state.search_state.results.is_empty() {
        self.state.search_state.current_result = 
            (self.state.search_state.current_result + 1)
            % self.state.search_state.results.len();
        
        // AVANT: idx pourrait être hors limites si le graph a changé
        let idx = self.state.search_state.results[self.state.search_state.current_result];
        
        // APRÈS: Vérifier les limites
        let idx = self.state.search_state.results[self.state.search_state.current_result];
        if idx < self.state.graph.len() {
            self.state.selected_index = idx;
            self.auto_scroll();
        } else {
            // Invalider les résultats de recherche
            self.state.search_state.results.clear();
            self.state.set_flash_message("Résultats de recherche obsolètes".into());
        }
    }
}
```

---

## 5. Checklist de validation

Après avoir effectué toutes les corrections :

```bash
# 1. Vérifier que le code compile
cargo build

# 2. Vérifier qu'il n'y a plus de warnings
cargo clippy --all-features -- -D warnings

# 3. Vérifier le formatage
cargo fmt -- --check

# 4. Exécuter les tests
cargo test

# 5. Tester manuellement l'application
cargo run
```

---

## 6. Résumé des fichiers à modifier

| Fichier | Modifications |
|---------|---------------|
| `src/app.rs` | Supprimer 4 imports inutilisés |
| `src/ui/confirm_dialog.rs` | Supprimer import `Wrap` |
| `src/event.rs` | Préfixer variables, corriger bug recherche |
| `src/git/commit.rs` | Préfixer `sig` |
| `src/ui/nav_bar.rs` | Préfixer `i` ou supprimer enumerate |
| `src/ui/status_bar.rs` | Préfixer `repo_path` ou l'utiliser |
| `src/error.rs` | Supprimer variant `Terminal` |
| `src/git/blame.rs` | Supprimer/utiliser champs non lus |
| `src/git/branch.rs` | Supprimer/utiliser champs non lus |
| `src/ui/graph_view.rs` | Fix troncature hash |
| `src/ui/blame_view.rs` | Fix troncature UTF-8 |

---

## Notes pour le développeur

Ces corrections sont **non-bloquantes** mais importantes car :
1. Les warnings masquent les vrais problèmes
2. Le code mort augmente la charge cognitive
3. Les bugs de troncature UTF-8 peuvent faire crasher l'application avec certains noms d'utilisateurs internationaux
