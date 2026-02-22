# STEP 03 - Extraction des Utilitaires Communs UI

**Priorité**: 🔴 Haute  
**Effort estimé**: 3-4 heures  
**Risque**: Moyen (refactoring transversal)  
**Prérequis**: STEP_01, STEP_02 complétés

---

## Objectif

Éliminer la duplication de code dans le module UI en extrayant les patterns communs dans des composants réutilisables. Cela réduira la charge de maintenance et améliorera la cohérence visuelle.

---

## 1. Problèmes identifiés

### Duplications majeures

| Pattern | Occurrences | Fichiers |
|---------|-------------|----------|
| `centered_rect()` | 5x | common, confirm_dialog, merge_picker, conflicts_view, loading |
| Status bar rendering | 3x | status_bar, staging_view, branches_view |
| List avec highlight | 8x | graph_view, files_view, branches_view (5x), staging_view, branch_panel |
| Border focus style | 10x | Presque tous les fichiers |
| Help bar | 3x | help_bar, staging_view, branches_view |

---

## 2. Restructuration du module `src/ui/common/`

### Structure cible

```
src/ui/common/
├── mod.rs              # Re-exports
├── rect.rs             # centered_rect et helpers de layout
├── style.rs            # Styles communs (focus, highlight, etc.)
├── list.rs             # StyledList component
├── block.rs            # StyledBlock builder
├── status_bar.rs       # StatusBar component unifié
├── help_bar.rs         # HelpBar configurable
├── popup.rs            # Popup/dialog base component
└── text.rs             # Helpers de troncature Unicode-safe
```

---

## 3. Implémentation des composants

### 3.1. `src/ui/common/rect.rs`

```rust
//! Utilitaires de calcul de zones rectangulaires.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Crée un rectangle centré dans la zone donnée.
///
/// # Arguments
/// * `percent_x` - Pourcentage de largeur (0-100)
/// * `percent_y` - Pourcentage de hauteur (0-100)
/// * `area` - Zone parente
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical_layout[1])[1]
}

/// Crée un rectangle centré avec dimensions fixes.
pub fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    
    Rect::new(
        x,
        y,
        width.min(area.width),
        height.min(area.height),
    )
}

/// Vérifie si le terminal est suffisamment grand.
pub fn is_terminal_size_adequate(area: Rect, min_width: u16, min_height: u16) -> bool {
    area.width >= min_width && area.height >= min_height
}
```

### 3.2. `src/ui/common/style.rs`

```rust
//! Styles communs pour l'interface.

use ratatui::style::{Color, Modifier, Style};

/// Couleur de bordure quand un panel a le focus.
pub const FOCUS_COLOR: Color = Color::Cyan;

/// Couleur de bordure inactive.
pub const INACTIVE_COLOR: Color = Color::DarkGray;

/// Retourne le style de bordure selon l'état de focus.
pub fn border_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(FOCUS_COLOR)
    } else {
        Style::default().fg(INACTIVE_COLOR)
    }
}

/// Style pour les éléments sélectionnés dans une liste.
pub fn highlight_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

/// Style pour les titres de section.
pub fn title_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Style pour le texte désactivé/secondaire.
pub fn dim_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// Style pour les messages d'erreur.
pub fn error_style() -> Style {
    Style::default().fg(Color::Red)
}

/// Style pour les messages de succès.
pub fn success_style() -> Style {
    Style::default().fg(Color::Green)
}

/// Style pour les ajouts dans les diffs.
pub fn diff_add_style() -> Style {
    Style::default().fg(Color::Green)
}

/// Style pour les suppressions dans les diffs.
pub fn diff_remove_style() -> Style {
    Style::default().fg(Color::Red)
}

/// Style pour les headers dans les diffs.
pub fn diff_header_style() -> Style {
    Style::default().fg(Color::Cyan)
}
```

### 3.3. `src/ui/common/block.rs`

```rust
//! Builder pour les blocs stylisés.

use ratatui::{
    style::Style,
    widgets::{Block, Borders},
};
use super::style::{border_style, title_style};

/// Builder pour créer des blocs avec un style cohérent.
pub struct StyledBlock {
    title: String,
    is_focused: bool,
    borders: Borders,
}

impl StyledBlock {
    /// Crée un nouveau builder de bloc.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            is_focused: false,
            borders: Borders::ALL,
        }
    }

    /// Définit l'état de focus.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Définit les bordures à afficher.
    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    /// Construit le widget Block.
    pub fn build(self) -> Block<'static> {
        Block::default()
            .title(self.title)
            .title_style(title_style())
            .borders(self.borders)
            .border_style(border_style(self.is_focused))
    }
}
```

### 3.4. `src/ui/common/list.rs`

```rust
//! Composant liste stylisé et réutilisable.

use ratatui::{
    style::Style,
    widgets::{Block, List, ListItem, ListState},
    Frame,
    layout::Rect,
};
use super::{block::StyledBlock, style::highlight_style};

/// Configuration pour une liste stylisée.
pub struct StyledList<'a> {
    items: Vec<ListItem<'a>>,
    title: String,
    is_focused: bool,
    selected: Option<usize>,
}

impl<'a> StyledList<'a> {
    /// Crée une nouvelle liste.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            items: Vec::new(),
            title: title.into(),
            is_focused: false,
            selected: None,
        }
    }

    /// Définit les éléments de la liste.
    pub fn items(mut self, items: Vec<ListItem<'a>>) -> Self {
        self.items = items;
        self
    }

    /// Définit l'état de focus.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Définit l'index sélectionné.
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Rend la liste dans le frame.
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let block = StyledBlock::new(&self.title)
            .focused(self.is_focused)
            .build();

        let list = List::new(self.items)
            .block(block)
            .highlight_style(highlight_style());

        let mut state = ListState::default().with_selected(self.selected);
        frame.render_stateful_widget(list, area, &mut state);
    }
}

/// Helper pour créer des ListItem avec style cohérent.
pub fn list_item(content: impl Into<String>) -> ListItem<'static> {
    ListItem::new(content.into())
}

pub fn list_item_styled(content: impl Into<String>, style: Style) -> ListItem<'static> {
    ListItem::new(content.into()).style(style)
}
```

### 3.5. `src/ui/common/text.rs`

```rust
//! Utilitaires de manipulation de texte Unicode-safe.

/// Tronque une chaîne de manière safe pour Unicode.
///
/// # Arguments
/// * `s` - Chaîne à tronquer
/// * `max_len` - Longueur maximale en caractères
/// * `ellipsis` - Ajouter "…" si tronqué
pub fn truncate(s: &str, max_len: usize, ellipsis: bool) -> String {
    let char_count = s.chars().count();
    
    if char_count <= max_len {
        s.to_string()
    } else if ellipsis && max_len > 1 {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    } else {
        s.chars().take(max_len).collect()
    }
}

/// Tronque une chaîne au début (garde la fin).
pub fn truncate_start(s: &str, max_len: usize, ellipsis: bool) -> String {
    let char_count = s.chars().count();
    
    if char_count <= max_len {
        s.to_string()
    } else if ellipsis && max_len > 1 {
        let skip = char_count - max_len + 1;
        let truncated: String = s.chars().skip(skip).collect();
        format!("…{}", truncated)
    } else {
        s.chars().skip(char_count - max_len).collect()
    }
}

/// Pad une chaîne à droite jusqu'à la longueur spécifiée.
pub fn pad_right(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - char_count))
    }
}

/// Pad une chaîne à gauche jusqu'à la longueur spécifiée.
pub fn pad_left(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat(width - char_count), s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10, true), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 5, true), "hell…");
        assert_eq!(truncate("hello world", 5, false), "hello");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("héllo wörld", 5, true), "héll…");
    }

    #[test]
    fn test_truncate_start() {
        assert_eq!(truncate_start("/a/very/long/path/file.rs", 15, true), "…ong/path/file.rs");
    }
}
```

### 3.6. `src/ui/common/popup.rs`

```rust
//! Composant de base pour les popups et dialogues.

use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use super::{rect::centered_rect, style::border_style};

/// Configuration pour un popup.
pub struct Popup<'a> {
    title: &'a str,
    content: Vec<Line<'a>>,
    width_percent: u16,
    height_percent: u16,
    is_focused: bool,
}

impl<'a> Popup<'a> {
    /// Crée un nouveau popup.
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            content: Vec::new(),
            width_percent: 60,
            height_percent: 40,
            is_focused: true,
        }
    }

    /// Définit le contenu du popup.
    pub fn content(mut self, content: Vec<Line<'a>>) -> Self {
        self.content = content;
        self
    }

    /// Définit la taille en pourcentage.
    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width_percent = width;
        self.height_percent = height;
        self
    }

    /// Définit l'état de focus.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Rend le popup dans le frame.
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let popup_area = centered_rect(self.width_percent, self.height_percent, area);
        
        // Clear le fond
        frame.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(border_style(self.is_focused));
        
        let paragraph = Paragraph::new(self.content)
            .block(block)
            .wrap(Wrap { trim: true });
        
        frame.render_widget(paragraph, popup_area);
    }

    /// Retourne la zone du popup pour un rendu personnalisé.
    pub fn area(&self, parent: Rect) -> Rect {
        centered_rect(self.width_percent, self.height_percent, parent)
    }
}
```

### 3.7. `src/ui/common/help_bar.rs`

```rust
//! Barre d'aide configurable.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Un raccourci clavier à afficher.
pub struct KeyBinding {
    pub key: &'static str,
    pub description: &'static str,
}

impl KeyBinding {
    pub const fn new(key: &'static str, description: &'static str) -> Self {
        Self { key, description }
    }
}

/// Configuration de la barre d'aide.
pub struct HelpBar<'a> {
    bindings: &'a [KeyBinding],
    separator: &'a str,
}

impl<'a> HelpBar<'a> {
    /// Crée une nouvelle barre d'aide.
    pub fn new(bindings: &'a [KeyBinding]) -> Self {
        Self {
            bindings,
            separator: "  ",
        }
    }

    /// Définit le séparateur entre les bindings.
    pub fn separator(mut self, sep: &'a str) -> Self {
        self.separator = sep;
        self
    }

    /// Rend la barre d'aide.
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let mut spans = Vec::new();
        
        for (i, binding) in self.bindings.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(self.separator));
            }
            
            spans.push(Span::styled(
                binding.key,
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::raw(": "));
            spans.push(Span::raw(binding.description));
        }
        
        let help_line = Line::from(spans);
        let help_paragraph = Paragraph::new(help_line);
        
        frame.render_widget(help_paragraph, area);
    }
}

// Bindings communs réutilisables
pub mod bindings {
    use super::KeyBinding;
    
    pub const QUIT: KeyBinding = KeyBinding::new("q", "Quitter");
    pub const HELP: KeyBinding = KeyBinding::new("?", "Aide");
    pub const NAV_UP_DOWN: KeyBinding = KeyBinding::new("↑↓", "Naviguer");
    pub const ENTER: KeyBinding = KeyBinding::new("Enter", "Sélectionner");
    pub const ESC: KeyBinding = KeyBinding::new("Esc", "Retour");
    pub const TAB: KeyBinding = KeyBinding::new("Tab", "Changer panel");
}
```

### 3.8. `src/ui/common/mod.rs`

```rust
//! Composants UI communs et réutilisables.

pub mod rect;
pub mod style;
pub mod block;
pub mod list;
pub mod text;
pub mod popup;
pub mod help_bar;

// Re-exports pour un accès plus simple
pub use rect::centered_rect;
pub use style::{border_style, highlight_style};
pub use block::StyledBlock;
pub use list::StyledList;
pub use text::truncate;
pub use popup::Popup;
pub use help_bar::{HelpBar, KeyBinding};
```

---

## 4. Migration des fichiers existants

### 4.1. `src/ui/confirm_dialog.rs`

```rust
// AVANT
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    // ... 20 lignes dupliquées
}

// APRÈS
use crate::ui::common::{centered_rect, Popup};

pub fn render_confirm_dialog(
    frame: &mut Frame,
    area: Rect,
    message: &str,
    confirm_key: &str,
    cancel_key: &str,
) {
    let content = vec![
        Line::from(message),
        Line::from(""),
        Line::from(vec![
            Span::styled(confirm_key, Style::default().fg(Color::Green)),
            Span::raw(" pour confirmer, "),
            Span::styled(cancel_key, Style::default().fg(Color::Red)),
            Span::raw(" pour annuler"),
        ]),
    ];
    
    Popup::new(" Confirmation ")
        .content(content)
        .size(50, 20)
        .render(frame, area);
}
```

### 4.2. `src/ui/branches_view.rs`

```rust
// AVANT
let border_style = if is_focused {
    Style::default().fg(Color::Cyan)
} else {
    Style::default()
};

let list = List::new(items)
    .block(Block::default().title("Branches").borders(Borders::ALL).border_style(border_style))
    .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

// APRÈS
use crate::ui::common::{StyledList, StyledBlock};

StyledList::new("Branches")
    .items(items)
    .focused(is_focused)
    .selected(Some(selected_index))
    .render(frame, area);
```

### 4.3. `src/ui/graph_view.rs`

```rust
// AVANT
let short_hash = &hash[..7];  // ❌ Peut paniquer

// APRÈS
use crate::ui::common::text::truncate;

let short_hash = truncate(&hash, 7, false);
```

---

## 5. Checklist de validation

```bash
# 1. Créer tous les fichiers dans src/ui/common/
ls -la src/ui/common/

# 2. Compiler
cargo build

# 3. Vérifier qu'il n'y a plus de duplication
grep -r "fn centered_rect" src/ui/ | wc -l  # Devrait être 1

# 4. Exécuter les tests
cargo test

# 5. Vérifier clippy
cargo clippy --all-features -- -D warnings

# 6. Test visuel de l'application
cargo run
```

---

## 6. Ordre de migration recommandé

1. **Créer** `src/ui/common/mod.rs` et tous les sous-modules
2. **Migrer** `centered_rect` (le plus dupliqué)
3. **Migrer** les styles communs (`border_style`, `highlight_style`)
4. **Migrer** les fichiers simples (`confirm_dialog.rs`, `loading.rs`)
5. **Migrer** les vues complexes (`branches_view.rs`, `staging_view.rs`)
6. **Supprimer** le code dupliqué dans chaque fichier migré
7. **Tester** chaque vue après migration

---

## Bénéfices attendus

| Métrique | Avant | Après |
|----------|-------|-------|
| Lignes dupliquées | ~200 | 0 |
| Fichiers avec `centered_rect` | 5 | 1 |
| Patterns de style incohérents | ~10 | 0 |
| Temps pour ajouter une nouvelle vue | Élevé | Réduit |
