# STEP 08 - Nouvelles Fonctionnalités

**Priorité**: 🟢 Basse  
**Effort estimé**: Variable selon la feature  
**Risque**: Variable  
**Prérequis**: STEP_01 à STEP_07 complétés

---

## Objectif

Proposer des nouvelles fonctionnalités pour améliorer l'expérience utilisateur de git_sv. Ces features sont classées par priorité et difficulté.

---

## 1. Fonctionnalités Prioritaires (Quick Wins)

### 1.1 🎯 Raccourcis personnalisables

**Effort**: 2-3h | **Impact**: Élevé

Permettre aux utilisateurs de personnaliser les raccourcis clavier.

```rust
// src/config/keybindings.rs

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    pub quit: Vec<String>,
    pub move_up: Vec<String>,
    pub move_down: Vec<String>,
    pub stage_file: Vec<String>,
    // ...
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            quit: vec!["q".into(), "Ctrl+c".into()],
            move_up: vec!["k".into(), "Up".into()],
            move_down: vec!["j".into(), "Down".into()],
            stage_file: vec!["s".into()],
            // ...
        }
    }
}

// Fichier de config: ~/.config/git_sv/keybindings.toml
// [keybindings]
// quit = ["q", "Ctrl+c"]
// move_up = ["k", "Up", "Ctrl+p"]
```

---

### 1.2 🎯 Thèmes personnalisables

**Effort**: 2h | **Impact**: Moyen

Supporter différents thèmes de couleurs.

```rust
// src/config/theme.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub foreground: String,
    pub selection: String,
    pub diff_add: String,
    pub diff_remove: String,
    pub branch_colors: Vec<String>,
    // ...
}

// Thèmes prédéfinis
pub fn theme_dark() -> Theme { ... }
pub fn theme_light() -> Theme { ... }
pub fn theme_solarized() -> Theme { ... }
pub fn theme_nord() -> Theme { ... }
```

---

### 1.3 🎯 Filtrage du graph par auteur/date

**Effort**: 3h | **Impact**: Élevé

```rust
// Dans SearchState, ajouter:
pub struct GraphFilter {
    pub author: Option<String>,
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
    pub path: Option<String>,
    pub message_contains: Option<String>,
}

// Keybinding: 'f' pour ouvrir le filtre
// UI: Popup avec champs de filtre
```

---

### 1.4 🎯 Diff side-by-side

**Effort**: 4h | **Impact**: Élevé

Afficher les diffs en mode côte-à-côte au lieu de unified.

```rust
// src/ui/diff_view.rs

pub enum DiffViewMode {
    Unified,      // Mode actuel
    SideBySide,   // Nouveau mode
}

fn render_side_by_side_diff(
    frame: &mut Frame,
    area: Rect,
    diff: &FileDiff,
    scroll: usize,
) {
    // Split horizontal en deux
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Gauche: ancien fichier
    render_diff_side(frame, chunks[0], &diff.old_lines, "Ancien");

    // Droite: nouveau fichier
    render_diff_side(frame, chunks[1], &diff.new_lines, "Nouveau");
}
```

---

## 2. Fonctionnalités Intermédiaires

### 2.1 📌 Rebase interactif

**Effort**: 8h | **Impact**: Très élevé

Supporter le rebase interactif depuis l'interface.

```rust
// src/git/rebase.rs

pub struct RebaseState {
    pub commits: Vec<RebaseCommit>,
    pub current_step: usize,
    pub original_head: Oid,
}

pub struct RebaseCommit {
    pub oid: Oid,
    pub action: RebaseAction,
    pub message: String,
}

pub enum RebaseAction {
    Pick,
    Reword,
    Edit,
    Squash,
    Fixup,
    Drop,
}

// UI: Liste des commits avec possibilité de:
// - Réordonner (drag & drop via j/k + Enter)
// - Changer l'action (p/r/e/s/f/d)
// - Éditer le message (pour reword)
```

**Keybindings**:

- `r` sur un commit → Ouvrir le menu rebase
- Dans le mode rebase:
  - `p` → Pick
  - `r` → Reword
  - `s` → Squash
  - `f` → Fixup
  - `d` → Drop
  - `Ctrl+Enter` → Exécuter le rebase

---

### 2.2 📌 Git bisect interactif

**Effort**: 6h | **Impact**: Moyen

Interface pour git bisect.

```rust
// src/git/bisect.rs

pub struct BisectState {
    pub good_commit: Option<Oid>,
    pub bad_commit: Option<Oid>,
    pub current_commit: Oid,
    pub remaining_steps: usize,
    pub history: Vec<BisectStep>,
}

pub struct BisectStep {
    pub commit: Oid,
    pub verdict: BisectVerdict,
}

pub enum BisectVerdict {
    Good,
    Bad,
    Skip,
}
```

**Keybindings**:

- `B` (majuscule) → Démarrer bisect
- Dans le mode bisect:
  - `g` → Mark good
  - `b` → Mark bad
  - `s` → Skip
  - `q` → Abort bisect

---

### 2.3 📌 Git reflog

**Effort**: 3h | **Impact**: Moyen

Afficher et naviguer dans le reflog.

```rust
// src/git/reflog.rs

pub struct ReflogEntry {
    pub oid: Oid,
    pub message: String,
    pub action: String,  // "commit", "checkout", "rebase", etc.
    pub timestamp: i64,
}

pub fn get_reflog(repo: &Repository, limit: usize) -> Result<Vec<ReflogEntry>> {
    let reflog = repo.reflog("HEAD")?;
    // ...
}
```

**UI**: Nouvelle vue accessible via `R` (majuscule)

---

### 2.4 📌 Support des submodules

**Effort**: 5h | **Impact**: Moyen

Afficher et gérer les submodules.

```rust
// src/git/submodule.rs

pub struct SubmoduleInfo {
    pub name: String,
    pub path: String,
    pub url: String,
    pub current_commit: Option<Oid>,
    pub status: SubmoduleStatus,
}

pub enum SubmoduleStatus {
    UpToDate,
    Modified,
    Uninitialized,
    OutOfSync,
}

// Actions possibles:
// - Init/deinit
// - Update
// - Sync
// - Open in new instance
```

---

### 2.5 📌 Git hooks viewer

**Effort**: 2h | **Impact**: Faible

Voir et éditer les hooks git.

```rust
// src/git/hooks.rs

pub struct GitHook {
    pub name: String,  // "pre-commit", "post-commit", etc.
    pub path: PathBuf,
    pub is_enabled: bool,
    pub content: Option<String>,
}

pub fn list_hooks(repo: &Repository) -> Vec<GitHook> {
    let hooks_dir = repo.path().join("hooks");
    // ...
}
```

---

## 3. Améliorations UX

### 3.1 🎨 Animations

**Effort**: 3h | **Impact**: Faible (cosmétique)

Ajouter des transitions douces.

```rust
// Animation du scroll
pub struct AnimatedValue {
    current: f32,
    target: f32,
    velocity: f32,
}

impl AnimatedValue {
    pub fn update(&mut self, dt: f32) {
        // Interpolation smooth
        self.current += (self.target - self.current) * dt * 10.0;
    }
}
```

---

### 3.2 🎨 Indicateurs visuels améliorés

**Effort**: 2h | **Impact**: Moyen

- Badge de notifications (PRs, issues)
- Indicateur de push/pull nécessaire
- Alerte pour les commits non pushés

```rust
// src/ui/indicators.rs

pub struct StatusIndicators {
    pub commits_ahead: usize,
    pub commits_behind: usize,
    pub has_stashes: bool,
    pub has_conflicts: bool,
    pub pr_count: Option<usize>,
}
```

---

### 3.3 🎨 Raccourcis contextuels

**Effort**: 2h | **Impact**: Moyen

Afficher uniquement les raccourcis pertinents selon le contexte.

```rust
// Dans la help bar, filtrer selon:
// - ViewMode actuel
// - État du repo (has_staged, has_conflicts, etc.)
// - Focus actuel
```

---

## 4. Priorités recommandées

### Phase 1 (Quick wins - 1-2 semaines)

1. Filtrage du graph ⭐⭐⭐
2. Diff side-by-side ⭐⭐⭐
3. Thèmes personnalisables ⭐⭐

### Phase 2 (Valeur ajoutée - 2-4 semaines)

1. Rebase interactif ⭐⭐⭐⭐⭐
2. Git reflog ⭐⭐⭐
3. Raccourcis personnalisables ⭐⭐

---

## Notes pour l'implémentation

1. **Feature flags**: Utiliser des feature flags Cargo pour les fonctionnalités optionnelles

   ```toml
   [features]
   github = ["reqwest", "serde_json"]
   plugins = ["mlua"]
   ```

2. **Documentation**: Chaque nouvelle feature doit inclure:
   - Doc dans README
   - Tests
   - Exemple d'utilisation

3. **Rétrocompatibilité**: Les fichiers de config doivent avoir des valeurs par défaut pour les anciennes versions
