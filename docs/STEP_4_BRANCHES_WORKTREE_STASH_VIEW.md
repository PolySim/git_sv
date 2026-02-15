# STEP 4 — Vue Branches / Worktrees / Stash

## Objectif

Créer une vue dédiée à la gestion des branches, worktrees et stashes, accessible via la
touche `3` depuis n'importe quelle vue. Cette vue permet de :
- Voir toutes les branches (locales et remote) et naviguer entre elles
- Créer, supprimer, checkout des branches
- Voir et gérer les worktrees
- Voir et gérer les stashes (apply, pop, drop)

---

## Prérequis

- **STEP 3** doit être implémenté (système de `ViewMode` multi-vues et navigation `1`/`2`/`3`).

---

## Plan d'implémentation

### 4.1 — Étendre le module git pour les worktrees

**Nouveau fichier** : `src/git/worktree.rs`

```rust
use git2::Repository;
use crate::error::Result;

/// Informations sur un worktree.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Nom du worktree.
    pub name: String,
    /// Chemin absolu du worktree.
    pub path: String,
    /// Branche associée (si applicable).
    pub branch: Option<String>,
    /// Est-ce le worktree principal ?
    pub is_main: bool,
}

/// Liste tous les worktrees du repository.
pub fn list_worktrees(repo: &Repository) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();

    // Worktree principal.
    if let Some(path) = repo.workdir() {
        let branch = repo.head().ok()
            .and_then(|h| h.shorthand().map(String::from));
        worktrees.push(WorktreeInfo {
            name: "main".to_string(),
            path: path.display().to_string(),
            branch,
            is_main: true,
        });
    }

    // Worktrees additionnels.
    let wt_names = repo.worktrees()?;
    for name in wt_names.iter() {
        if let Some(name) = name {
            if let Ok(wt) = repo.find_worktree(name) {
                let path = wt.path().display().to_string();
                // Tenter d'ouvrir le worktree pour lire sa branche.
                let branch = Repository::open(&path).ok()
                    .and_then(|r| r.head().ok())
                    .and_then(|h| h.shorthand().map(String::from));

                worktrees.push(WorktreeInfo {
                    name: name.to_string(),
                    path,
                    branch,
                    is_main: false,
                });
            }
        }
    }

    Ok(worktrees)
}

/// Crée un nouveau worktree.
pub fn create_worktree(repo: &Repository, name: &str, path: &str, branch: Option<&str>) -> Result<()> {
    let reference = if let Some(branch_name) = branch {
        let refname = format!("refs/heads/{}", branch_name);
        Some(repo.find_reference(&refname)?)
    } else {
        None
    };

    repo.worktree(name, std::path::Path::new(path), None)?;
    Ok(())
}

/// Supprime un worktree (prune).
pub fn remove_worktree(repo: &Repository, name: &str) -> Result<()> {
    let wt = repo.find_worktree(name)?;
    // Vérifier que le worktree est prunable.
    if wt.validate().is_ok() {
        wt.prune(None)?;
    }
    Ok(())
}
```

**Fichier** : `src/git/mod.rs` — Ajouter `pub mod worktree;`

**Fichier** : `src/git/repo.rs` — Ajouter les méthodes wrapper :

```rust
use super::worktree::WorktreeInfo;

impl GitRepo {
    pub fn worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        super::worktree::list_worktrees(&self.repo)
    }

    pub fn create_worktree(&self, name: &str, path: &str, branch: Option<&str>) -> Result<()> {
        super::worktree::create_worktree(&self.repo, name, path, branch)
    }

    pub fn remove_worktree(&self, name: &str) -> Result<()> {
        super::worktree::remove_worktree(&self.repo, name)
    }
}
```

---

### 4.2 — Étendre le module branches

**Fichier** : `src/git/branch.rs`

Ajouter le support des branches remote :

```rust
/// Liste toutes les branches (locales et remote).
pub fn list_all_branches(repo: &Repository) -> Result<Vec<BranchInfo>> {
    let mut branches = Vec::new();

    let head_ref = repo.head().ok();
    let head_name = head_ref.as_ref()
        .and_then(|h| h.shorthand().map(String::from));

    // Branches locales.
    for branch_result in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch_result?;
        let name = branch.name()?.unwrap_or("???").to_string();
        let is_head = head_name.as_deref() == Some(&name);
        branches.push(BranchInfo { name, is_head, is_remote: false });
    }

    // Branches remote.
    for branch_result in repo.branches(Some(BranchType::Remote))? {
        let (branch, _) = branch_result?;
        let name = branch.name()?.unwrap_or("???").to_string();
        branches.push(BranchInfo { name, is_head: false, is_remote: true });
    }

    Ok(branches)
}
```

Ajouter les métadonnées supplémentaires :

```rust
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    /// Dernier message de commit sur cette branche.
    pub last_commit_message: Option<String>,
    /// Date du dernier commit.
    pub last_commit_date: Option<i64>,
    /// Nombre de commits d'avance/retard par rapport à la branche tracking.
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
}
```

---

### 4.3 — Étendre le module stash

**Fichier** : `src/git/stash.rs`

Ajouter le détail d'un stash :

```rust
/// Applique un stash sans le supprimer.
pub fn apply_stash(repo: &mut Repository, index: usize) -> Result<()> {
    let mut opts = git2::StashApplyOptions::new();
    repo.stash_apply(index, Some(&mut opts))?;
    Ok(())
}
```

Ajouter des métadonnées enrichies :

```rust
#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    /// Branche sur laquelle le stash a été créé.
    pub branch: Option<String>,
    /// Date de création du stash.
    pub timestamp: Option<i64>,
}
```

---

### 4.4 — État de la vue Branches

**Fichier** : `src/app.rs`

```rust
/// Section active dans la vue branches.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchesSection {
    Branches,
    Worktrees,
    Stashes,
}

/// Panneau focalisé dans la vue branches.
#[derive(Debug, Clone, PartialEq)]
pub enum BranchesFocus {
    /// Liste (gauche).
    List,
    /// Détail / Preview (droite).
    Detail,
    /// Input (création de branche, etc.).
    Input,
}

/// État de la vue branches/worktree/stash.
pub struct BranchesViewState {
    /// Section active (onglet).
    pub section: BranchesSection,
    /// Panneau focalisé.
    pub focus: BranchesFocus,

    // -- Branches --
    /// Branches locales.
    pub local_branches: Vec<BranchInfo>,
    /// Branches remote.
    pub remote_branches: Vec<BranchInfo>,
    /// Index sélectionné dans les branches locales.
    pub branch_selected: usize,
    /// Afficher les remote ? (toggle).
    pub show_remote: bool,

    // -- Worktrees --
    /// Liste des worktrees.
    pub worktrees: Vec<WorktreeInfo>,
    /// Index sélectionné.
    pub worktree_selected: usize,

    // -- Stashes --
    /// Liste des stashes.
    pub stashes: Vec<StashEntry>,
    /// Index sélectionné.
    pub stash_selected: usize,

    // -- Input --
    /// Texte de saisie (pour créer branche/worktree).
    pub input_text: String,
    /// Position du curseur.
    pub input_cursor: usize,
    /// Type d'action en cours de saisie.
    pub input_action: Option<InputAction>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    CreateBranch,
    CreateWorktree,
    RenameBranch,
    SaveStash,
}
```

---

### 4.5 — Actions spécifiques à la vue Branches

```rust
pub enum AppAction {
    // ... actions existantes + celles du STEP 3 ...

    // Navigation sections
    /// Basculer vers la section suivante (Branches → Worktrees → Stashes).
    NextSection,
    /// Basculer vers la section précédente.
    PrevSection,

    // Actions branches
    /// Checkout la branche sélectionnée.
    BranchCheckout,
    /// Créer une nouvelle branche (ouvre input).
    BranchCreate,
    /// Supprimer la branche sélectionnée.
    BranchDelete,
    /// Renommer la branche sélectionnée (ouvre input).
    BranchRename,
    /// Toggle affichage branches remote.
    ToggleRemoteBranches,

    // Actions worktrees
    /// Créer un worktree (ouvre input).
    WorktreeCreate,
    /// Supprimer le worktree sélectionné.
    WorktreeRemove,

    // Actions stashes
    /// Appliquer le stash sélectionné (sans supprimer).
    StashApply,
    /// Pop le stash sélectionné (appliquer + supprimer).
    StashPop,
    /// Supprimer le stash sélectionné.
    StashDrop,
    /// Créer un nouveau stash (ouvre input).
    StashSave,

    // Input
    InsertChar(char),
    DeleteChar,
    MoveCursorLeft,
    MoveCursorRight,
    ConfirmInput,
    CancelInput,
}
```

---

### 4.6 — Layout de la vue Branches

**Nouveau fichier** : `src/ui/branches_layout.rs`

```
┌──────────────────────────────────────────────────────────────┐
│  Status Bar — git_sv · branches · main                        │
├──────────────────────────────────────────────────────────────┤
│  [Branches]  [Worktrees]  [Stashes]     ← onglets (Tab)     │
├────────────────────────────┬─────────────────────────────────┤
│                            │                                 │
│  Branches locales          │  Détail de la branche           │
│  ─────────────────         │  ─────────────────────          │
│  * main          3↑ 0↓    │  Nom: feature/auth              │
│    feature/auth  0↑ 2↓    │  Dernier commit: abc1234        │
│    feature/ui    1↑ 0↓    │  "feat: ajout login"            │
│    hotfix/bug    0↑ 5↓    │  Date: 2026-02-14 15:30         │
│                            │  Ahead: 0  Behind: 2            │
│  Branches remote           │                                 │
│  ──────────────            │  Commits récents:               │
│    origin/main             │  ● abc1234 feat: ajout login    │
│    origin/develop          │  ● def5678 feat: ajout register │
│                            │  ● ghi9012 refactor: auth       │
│                            │                                 │
├────────────────────────────┴─────────────────────────────────┤
│  Tab:section  Enter:checkout  n:new  d:delete  r:rename      │
│  R:toggle remote  1:graph  2:staging  ?:aide                  │
└──────────────────────────────────────────────────────────────┘
```

Pour la section **Worktrees** :

```
├────────────────────────────┬─────────────────────────────────┤
│                            │                                 │
│  Worktrees                 │  Détail du worktree             │
│  ─────────────             │  ─────────────────              │
│  ● main (principal)        │  Nom: feature-wt                │
│    feature-wt              │  Chemin: /home/user/feature-wt  │
│                            │  Branche: feature/auth          │
│                            │                                 │
├────────────────────────────┴─────────────────────────────────┤
│  Tab:section  n:new  d:delete  1:graph  2:staging            │
```

Pour la section **Stashes** :

```
├────────────────────────────┬─────────────────────────────────┤
│                            │                                 │
│  Stashes                   │  Détail du stash                │
│  ────────                  │  ──────────────                 │
│  stash@{0}: WIP on main   │  Message: WIP on main           │
│  stash@{1}: feat: wip     │  Branche: main                  │
│                            │  Date: 2026-02-14 14:00         │
│                            │                                 │
│                            │  Fichiers modifiés:             │
│                            │  M src/app.rs                   │
│                            │  M src/ui/mod.rs                │
│                            │                                 │
├────────────────────────────┴─────────────────────────────────┤
│  Tab:section  a:apply  p:pop  d:drop  s:save  1:graph        │
```

#### Structure du layout

```rust
pub struct BranchesLayout {
    pub status_bar: Rect,
    pub tabs: Rect,
    pub list_panel: Rect,
    pub detail_panel: Rect,
    pub help_bar: Rect,
}

pub fn build_branches_layout(area: Rect) -> BranchesLayout {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Status bar
            Constraint::Length(1),   // Onglets
            Constraint::Min(0),      // Contenu
            Constraint::Length(2),   // Help bar
        ])
        .split(area);

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[2]);

    BranchesLayout {
        status_bar: outer[0],
        tabs: outer[1],
        list_panel: content[0],
        detail_panel: content[1],
        help_bar: outer[3],
    }
}
```

---

### 4.7 — Composant UI de la vue Branches

**Nouveau fichier** : `src/ui/branches_view.rs`

#### Rendu principal

```rust
pub fn render(
    frame: &mut Frame,
    state: &BranchesViewState,
    current_branch: &Option<String>,
    repo_path: &str,
    flash_message: Option<&str>,
) {
    let layout = branches_layout::build_branches_layout(frame.area());

    // Status bar.
    render_branches_status_bar(frame, current_branch, repo_path, flash_message, layout.status_bar);

    // Onglets.
    render_tabs(frame, &state.section, layout.tabs);

    // Contenu selon la section active.
    match state.section {
        BranchesSection::Branches => {
            render_branches_list(frame, state, layout.list_panel);
            render_branch_detail(frame, state, layout.detail_panel);
        }
        BranchesSection::Worktrees => {
            render_worktrees_list(frame, state, layout.list_panel);
            render_worktree_detail(frame, state, layout.detail_panel);
        }
        BranchesSection::Stashes => {
            render_stashes_list(frame, state, layout.list_panel);
            render_stash_detail(frame, state, layout.detail_panel);
        }
    }

    // Help bar contextuelle.
    render_branches_help(frame, &state.section, &state.focus, layout.help_bar);

    // Overlay d'input si actif.
    if state.focus == BranchesFocus::Input {
        render_input_overlay(frame, state, frame.area());
    }
}
```

#### Rendu des onglets

```rust
fn render_tabs(frame: &mut Frame, active: &BranchesSection, area: Rect) {
    let tabs = vec![
        ("Branches", BranchesSection::Branches),
        ("Worktrees", BranchesSection::Worktrees),
        ("Stashes", BranchesSection::Stashes),
    ];

    let mut spans = Vec::new();
    for (label, section) in &tabs {
        let style = if section == active {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {} ", label), style));
        spans.push(Span::raw("  "));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
```

#### Rendu de la liste des branches

```rust
fn render_branches_list(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    // Section locale.
    items.push(ListItem::new(Line::from(Span::styled(
        "Local",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ))));

    for (i, branch) in state.local_branches.iter().enumerate() {
        let prefix = if branch.is_head { "● " } else { "  " };
        let style = if branch.is_head {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let mut spans = vec![
            Span::styled(prefix, style),
            Span::styled(&branch.name, style),
        ];

        // Ahead/Behind si disponible.
        if let (Some(ahead), Some(behind)) = (branch.ahead, branch.behind) {
            spans.push(Span::styled(
                format!("  {}↑ {}↓", ahead, behind),
                Style::default().fg(Color::DarkGray),
            ));
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    // Section remote (si activée).
    if state.show_remote {
        items.push(ListItem::new(Line::from("")));
        items.push(ListItem::new(Line::from(Span::styled(
            "Remote",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))));

        for branch in &state.remote_branches {
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(&branch.name, Style::default().fg(Color::DarkGray)),
            ])));
        }
    }

    let list = List::new(items)
        .block(Block::default().title(" Branches ").borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut list_state = ListState::default();
    // Offset de +1 pour le header "Local".
    list_state.select(Some(state.branch_selected + 1));
    frame.render_stateful_widget(list, area, &mut list_state);
}
```

#### Overlay d'input (création branche, etc.)

```rust
fn render_input_overlay(frame: &mut Frame, state: &BranchesViewState, area: Rect) {
    let popup = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup);

    let title = match state.input_action {
        Some(InputAction::CreateBranch) => " Nouvelle branche ",
        Some(InputAction::RenameBranch) => " Renommer la branche ",
        Some(InputAction::CreateWorktree) => " Nouveau worktree ",
        Some(InputAction::SaveStash) => " Message du stash ",
        None => " Input ",
    };

    let paragraph = Paragraph::new(state.input_text.as_str())
        .block(Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)));

    frame.render_widget(paragraph, popup);

    // Curseur.
    frame.set_cursor_position((
        popup.x + state.input_cursor as u16 + 1,
        popup.y + 1,
    ));
}
```

---

### 4.8 — Keybindings de la vue Branches

**Fichier** : `src/ui/input.rs`

```rust
fn map_branches_key(key: KeyEvent, state: &BranchesViewState) -> Option<AppAction> {
    // Si on est en mode Input.
    if state.focus == BranchesFocus::Input {
        return match key.code {
            KeyCode::Enter => Some(AppAction::ConfirmInput),
            KeyCode::Esc => Some(AppAction::CancelInput),
            KeyCode::Char(c) => Some(AppAction::InsertChar(c)),
            KeyCode::Backspace => Some(AppAction::DeleteChar),
            KeyCode::Left => Some(AppAction::MoveCursorLeft),
            KeyCode::Right => Some(AppAction::MoveCursorRight),
            _ => None,
        };
    }

    // Navigation globale.
    match key.code {
        KeyCode::Char('1') => return Some(AppAction::SwitchToGraph),
        KeyCode::Char('2') => return Some(AppAction::SwitchToStaging),
        KeyCode::Tab => return Some(AppAction::NextSection),
        KeyCode::BackTab => return Some(AppAction::PrevSection),
        KeyCode::Char('q') => return Some(AppAction::Quit),
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp),
        _ => {}
    }

    // Actions par section.
    match state.section {
        BranchesSection::Branches => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Enter => Some(AppAction::BranchCheckout),
            KeyCode::Char('n') => Some(AppAction::BranchCreate),
            KeyCode::Char('d') => Some(AppAction::BranchDelete),
            KeyCode::Char('r') => Some(AppAction::BranchRename),
            KeyCode::Char('R') => Some(AppAction::ToggleRemoteBranches),
            _ => None,
        },
        BranchesSection::Worktrees => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('n') => Some(AppAction::WorktreeCreate),
            KeyCode::Char('d') => Some(AppAction::WorktreeRemove),
            _ => None,
        },
        BranchesSection::Stashes => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('a') => Some(AppAction::StashApply),
            KeyCode::Char('p') => Some(AppAction::StashPop),
            KeyCode::Char('d') => Some(AppAction::StashDrop),
            KeyCode::Char('s') => Some(AppAction::StashSave),
            _ => None,
        },
    }
}
```

---

### 4.9 — Logique des actions dans `app.rs`

```rust
AppAction::NextSection => {
    self.branches_view_state.section = match self.branches_view_state.section {
        BranchesSection::Branches => BranchesSection::Worktrees,
        BranchesSection::Worktrees => BranchesSection::Stashes,
        BranchesSection::Stashes => BranchesSection::Branches,
    };
    self.refresh_branches_view()?;
}

AppAction::BranchCreate => {
    self.branches_view_state.focus = BranchesFocus::Input;
    self.branches_view_state.input_action = Some(InputAction::CreateBranch);
    self.branches_view_state.input_text.clear();
    self.branches_view_state.input_cursor = 0;
}

AppAction::ConfirmInput => {
    match self.branches_view_state.input_action.take() {
        Some(InputAction::CreateBranch) => {
            let name = self.branches_view_state.input_text.clone();
            if !name.is_empty() {
                crate::git::branch::create_branch(&self.repo.repo, &name)?;
                self.set_flash_message(format!("Branche '{}' créée", name));
                self.refresh_branches_view()?;
            }
        }
        Some(InputAction::SaveStash) => {
            let msg = self.branches_view_state.input_text.clone();
            let msg = if msg.is_empty() { None } else { Some(msg.as_str()) };
            crate::git::stash::save_stash(&mut self.repo.repo, msg)?;
            self.set_flash_message("Stash sauvegardé".into());
            self.refresh_branches_view()?;
        }
        // ... autres actions
        _ => {}
    }
    self.branches_view_state.focus = BranchesFocus::List;
    self.branches_view_state.input_text.clear();
}

AppAction::StashPop => {
    if let Some(stash) = self.branches_view_state.stashes.get(self.branches_view_state.stash_selected) {
        let idx = stash.index;
        crate::git::stash::pop_stash(&mut self.repo.repo, idx)?;
        self.set_flash_message(format!("Stash @{{{}}} appliqué et supprimé", idx));
        self.refresh_branches_view()?;
    }
}

AppAction::BranchDelete => {
    if let Some(branch) = self.branches_view_state.local_branches.get(self.branches_view_state.branch_selected) {
        if branch.is_head {
            self.set_flash_message("Impossible de supprimer la branche courante".into());
        } else {
            let name = branch.name.clone();
            crate::git::branch::delete_branch(&self.repo.repo, &name)?;
            self.set_flash_message(format!("Branche '{}' supprimée", name));
            self.refresh_branches_view()?;
        }
    }
}
```

---

### 4.10 — Rafraîchissement de la vue

```rust
fn refresh_branches_view(&mut self) -> Result<()> {
    let all_branches = crate::git::branch::list_all_branches(&self.repo.repo)?;

    self.branches_view_state.local_branches = all_branches.iter()
        .filter(|b| !b.is_remote)
        .cloned()
        .collect();

    self.branches_view_state.remote_branches = all_branches.iter()
        .filter(|b| b.is_remote)
        .cloned()
        .collect();

    self.branches_view_state.worktrees = self.repo.worktrees().unwrap_or_default();

    // Note: stashes() nécessite &mut self car git2 le requiert.
    self.branches_view_state.stashes = self.repo.stashes().unwrap_or_default();

    // Réajuster les sélections.
    // ...

    Ok(())
}
```

---

### 4.11 — Intégrer dans la boucle de rendu

**Fichier** : `src/ui/mod.rs`

```rust
ViewMode::Branches => {
    branches_view::render(
        frame,
        &app.branches_view_state,
        &app.current_branch,
        &app.repo_path,
        app.flash_message.as_ref().map(|(msg, _)| msg.as_str()),
    );
}
```

---

### 4.12 — Supprimer l'ancien panneau de branches overlay

L'ancien `branch_panel.rs` (overlay) sera remplacé par cette vue dédiée.
Il peut être conservé temporairement et supprimé une fois la nouvelle vue stable.

Mettre à jour `AppAction::BranchList` pour basculer vers `ViewMode::Branches` au lieu
d'ouvrir l'overlay.

---

## Fichiers impactés

| Fichier | Modifications |
|---------|--------------|
| `src/git/worktree.rs` | **Nouveau fichier** — Opérations worktree |
| `src/git/mod.rs` | Ajouter `pub mod worktree` |
| `src/git/repo.rs` | Méthodes wrapper pour worktrees |
| `src/git/branch.rs` | `list_all_branches()`, métadonnées enrichies (`ahead`, `behind`, `last_commit`) |
| `src/git/stash.rs` | `apply_stash()`, métadonnées enrichies |
| `src/app.rs` | Nouveaux types (`BranchesSection`, `BranchesFocus`, `BranchesViewState`, `InputAction`), actions, logique |
| `src/ui/branches_view.rs` | **Nouveau fichier** — Vue complète branches/worktrees/stashes |
| `src/ui/branches_layout.rs` | **Nouveau fichier** — Layout de la vue |
| `src/ui/input.rs` | Keybindings pour la vue branches, saisie de texte |
| `src/ui/mod.rs` | Ajouter modules + dispatch rendu |
| `src/ui/branch_panel.rs` | **À supprimer** (remplacé par `branches_view.rs`) |

---

## Critères de validation

### Branches
- [ ] La touche `3` bascule vers la vue Branches
- [ ] Les branches locales sont listées avec indication de HEAD (●)
- [ ] Les branches remote sont affichables via toggle (`R`)
- [ ] Le ahead/behind est affiché pour chaque branche locale
- [ ] Le détail de la branche sélectionnée s'affiche à droite
- [ ] Checkout une branche avec `Enter`
- [ ] Créer une branche avec `n` (overlay de saisie)
- [ ] Supprimer une branche avec `d` (avec protection de HEAD)
- [ ] Renommer une branche avec `r`

### Worktrees
- [ ] Tab bascule vers l'onglet Worktrees
- [ ] Les worktrees existants sont listés
- [ ] Créer un worktree avec `n`
- [ ] Supprimer un worktree avec `d`

### Stashes
- [ ] Tab bascule vers l'onglet Stashes
- [ ] Les stashes sont listés avec leur message
- [ ] Apply un stash avec `a`
- [ ] Pop un stash avec `p`
- [ ] Drop un stash avec `d`
- [ ] Sauvegarder un stash avec `s` (overlay de saisie)

### Général
- [ ] Navigation fluide entre les 3 onglets avec Tab
- [ ] Les touches `1`/`2` permettent de revenir aux autres vues
- [ ] Les messages flash confirment chaque action
- [ ] `cargo clippy` ne génère aucun warning
