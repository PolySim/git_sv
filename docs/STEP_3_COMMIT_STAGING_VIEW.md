# STEP 3 — Vue Commit / Staging

## Objectif

Créer une vue dédiée au staging et à la création de commits, accessible via un raccourci
clavier depuis la vue principale. Cette vue permet de :
- Voir les fichiers modifiés / non suivis / staged
- Stage et unstage des fichiers individuellement ou en masse
- Voir le diff du fichier survolé en temps réel
- Saisir un message de commit et créer le commit

---

## Prérequis

- **STEP 2** doit être implémenté (diff viewer et `FileDiff`).

---

## Plan d'implémentation

### 3.1 — Nouveau `ViewMode` pour la vue staging

**Fichier** : `src/app.rs`

#### Étendre le `ViewMode`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Graph,
    Help,
    Staging,    // NOUVEAU
    Branches,   // NOUVEAU (pour STEP 4)
}
```

#### Raccourci clavier

La touche `2` (ou `c` quand le focus est global) basculera vers `ViewMode::Staging`.
La touche `1` revient à `ViewMode::Graph`.
La touche `3` ira vers `ViewMode::Branches` (STEP 4).

**Fichier** : `src/ui/input.rs`

```rust
// Navigation entre les vues principales.
KeyCode::Char('1') => Some(AppAction::SwitchToGraph),
KeyCode::Char('2') => Some(AppAction::SwitchToStaging),
KeyCode::Char('3') => Some(AppAction::SwitchToBranches),
```

---

### 3.2 — État de la vue Staging

**Fichier** : `src/app.rs`

#### Nouvelles structures

```rust
/// Panneau focalisé dans la vue staging.
#[derive(Debug, Clone, PartialEq)]
pub enum StagingFocus {
    /// Liste des fichiers non staged (working directory).
    Unstaged,
    /// Liste des fichiers staged (index).
    Staged,
    /// Panneau de diff (droite).
    Diff,
    /// Champ de saisie du message de commit.
    CommitMessage,
}

/// État de la vue staging.
pub struct StagingState {
    /// Fichiers non staged.
    pub unstaged_files: Vec<StatusEntry>,
    /// Fichiers staged.
    pub staged_files: Vec<StatusEntry>,
    /// Index sélectionné dans le panneau unstaged.
    pub unstaged_selected: usize,
    /// Index sélectionné dans le panneau staged.
    pub staged_selected: usize,
    /// Panneau actuellement focalisé.
    pub focus: StagingFocus,
    /// Diff du fichier survolé.
    pub current_diff: Option<FileDiff>,
    /// Offset de scroll dans le diff.
    pub diff_scroll: usize,
    /// Message de commit en cours de saisie.
    pub commit_message: String,
    /// Position du curseur dans le message.
    pub cursor_position: usize,
}
```

#### Nouveaux champs dans `App`

```rust
pub struct App {
    // ... champs existants ...
    pub staging_state: StagingState,
}
```

---

### 3.3 — Actions spécifiques à la vue Staging

**Fichier** : `src/app.rs`

```rust
pub enum AppAction {
    // ... actions existantes ...

    /// Basculer vers la vue Graph.
    SwitchToGraph,
    /// Basculer vers la vue Staging.
    SwitchToStaging,
    /// Basculer vers la vue Branches.
    SwitchToBranches,

    // Actions Staging
    /// Stage le fichier sélectionné.
    StageFile,
    /// Unstage le fichier sélectionné.
    UnstageFile,
    /// Stage tous les fichiers.
    StageAll,
    /// Unstage tous les fichiers.
    UnstageAll,
    /// Activer le mode saisie de message de commit.
    StartCommitMessage,
    /// Valider le commit.
    ConfirmCommit,
    /// Annuler la saisie du message.
    CancelCommitMessage,
}
```

#### Logique des actions

```rust
AppAction::StageFile => {
    if let Some(file) = self.staging_state.unstaged_files.get(self.staging_state.unstaged_selected) {
        crate::git::commit::stage_file(&self.repo.repo, &file.path)?;
        self.refresh_staging()?;
    }
}
AppAction::UnstageFile => {
    if let Some(file) = self.staging_state.staged_files.get(self.staging_state.staged_selected) {
        crate::git::commit::unstage_file(&self.repo.repo, &file.path)?;
        self.refresh_staging()?;
    }
}
AppAction::StageAll => {
    crate::git::commit::stage_all(&self.repo.repo)?;
    self.refresh_staging()?;
}
AppAction::ConfirmCommit => {
    if !self.staging_state.commit_message.is_empty() && !self.staging_state.staged_files.is_empty() {
        crate::git::commit::create_commit(&self.repo.repo, &self.staging_state.commit_message)?;
        self.staging_state.commit_message.clear();
        self.refresh_staging()?;
        self.set_flash_message("Commit créé avec succès".into());
    }
}
```

---

### 3.4 — Fonction unstage manquante

**Fichier** : `src/git/commit.rs`

Ajouter la fonction `unstage_file` qui n'existe pas encore :

```rust
/// Unstage un fichier (le retirer de l'index, revenir à HEAD).
pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    let head_tree = head_commit.tree()?;

    // Réinitialiser ce fichier dans l'index depuis HEAD.
    repo.reset_default(Some(&head_commit.as_object()), [path])?;
    Ok(())
}

/// Unstage tous les fichiers.
pub fn unstage_all(repo: &Repository) -> Result<()> {
    let head = repo.head()?;
    let obj = head.peel(git2::ObjectType::Commit)?;
    repo.reset(&obj, git2::ResetType::Mixed, None)?;
    Ok(())
}
```

---

### 3.5 — Séparation des fichiers staged/unstaged

**Fichier** : `src/app.rs`

```rust
/// Rafraîchit l'état de la vue staging.
fn refresh_staging(&mut self) -> Result<()> {
    let all_entries = self.repo.status().unwrap_or_default();

    self.staging_state.staged_files = all_entries.iter()
        .filter(|e| e.is_staged())
        .cloned()
        .collect();

    self.staging_state.unstaged_files = all_entries.iter()
        .filter(|e| e.is_unstaged())
        .cloned()
        .collect();

    // Réajuster les sélections.
    if self.staging_state.unstaged_selected >= self.staging_state.unstaged_files.len() {
        self.staging_state.unstaged_selected = self.staging_state.unstaged_files.len().saturating_sub(1);
    }
    if self.staging_state.staged_selected >= self.staging_state.staged_files.len() {
        self.staging_state.staged_selected = self.staging_state.staged_files.len().saturating_sub(1);
    }

    // Charger le diff du fichier survolé.
    self.load_staging_diff();

    Ok(())
}
```

**Fichier** : `src/git/repo.rs`

Ajouter des méthodes utilitaires à `StatusEntry` :

```rust
impl StatusEntry {
    pub fn is_staged(&self) -> bool {
        self.status.intersects(
            git2::Status::INDEX_NEW
            | git2::Status::INDEX_MODIFIED
            | git2::Status::INDEX_DELETED
            | git2::Status::INDEX_RENAMED
        )
    }

    pub fn is_unstaged(&self) -> bool {
        self.status.intersects(
            git2::Status::WT_MODIFIED
            | git2::Status::WT_DELETED
            | git2::Status::WT_NEW
            | git2::Status::WT_RENAMED
        )
    }
}
```

---

### 3.6 — Layout de la vue Staging

**Fichier** : `src/ui/staging_layout.rs` (nouveau fichier)

```
┌──────────────────────────────────────────────────────────┐
│  Status Bar (1 ligne) — git_sv · staging · main          │
├────────────────────────────┬─────────────────────────────┤
│  Unstaged (50%)            │                             │
│  ┌────────────────────────┐│                             │
│  │ M  src/app.rs          ││    Diff du fichier          │
│  │ M  src/ui/mod.rs       ││    sélectionné              │
│  │ ?  new_file.txt        ││                             │
│  └────────────────────────┘│    (avec coloration         │
│  Staged (50%)              │     +vert / -rouge)         │
│  ┌────────────────────────┐│                             │
│  │ A  src/git/worktree.rs ││                             │
│  │ M  Cargo.toml          ││                             │
│  └────────────────────────┘│                             │
├────────────────────────────┴─────────────────────────────┤
│  Message: feat: ajout de la vue staging█                 │
├──────────────────────────────────────────────────────────┤
│  1:graph  2:staging  3:branches  a:stage all  s:stage    │
│  u:unstage  Enter:commit  Esc:annuler  ?:aide            │
└──────────────────────────────────────────────────────────┘
```

#### Structure du layout

```rust
pub struct StagingLayout {
    pub status_bar: Rect,
    pub unstaged_panel: Rect,
    pub staged_panel: Rect,
    pub diff_panel: Rect,
    pub commit_message: Rect,
    pub help_bar: Rect,
}

pub fn build_staging_layout(area: Rect) -> StagingLayout {
    // Split vertical : status_bar + contenu + message + help_bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Status bar
            Constraint::Min(0),      // Contenu principal
            Constraint::Length(3),   // Zone message commit
            Constraint::Length(2),   // Help bar
        ])
        .split(area);

    // Split horizontal du contenu : listes (40%) + diff (60%)
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[1]);

    // Split vertical de la partie gauche : unstaged (50%) + staged (50%)
    let lists = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(content[0]);

    StagingLayout {
        status_bar: outer[0],
        unstaged_panel: lists[0],
        staged_panel: lists[1],
        diff_panel: content[1],
        commit_message: outer[2],
        help_bar: outer[3],
    }
}
```

---

### 3.7 — Composant UI de la vue Staging

**Nouveau fichier** : `src/ui/staging_view.rs`

#### Rendu principal

```rust
pub fn render(
    frame: &mut Frame,
    staging_state: &StagingState,
    current_branch: &Option<String>,
    repo_path: &str,
    flash_message: Option<&str>,
) {
    let layout = staging_layout::build_staging_layout(frame.area());

    // Status bar.
    render_staging_status_bar(frame, current_branch, repo_path, flash_message, layout.status_bar);

    // Panneau unstaged.
    render_file_list(
        frame,
        "Unstaged",
        &staging_state.unstaged_files,
        staging_state.unstaged_selected,
        staging_state.focus == StagingFocus::Unstaged,
        layout.unstaged_panel,
    );

    // Panneau staged.
    render_file_list(
        frame,
        "Staged",
        &staging_state.staged_files,
        staging_state.staged_selected,
        staging_state.focus == StagingFocus::Staged,
        layout.staged_panel,
    );

    // Panneau diff.
    diff_view::render(
        frame,
        staging_state.current_diff.as_ref(),
        staging_state.diff_scroll,
        layout.diff_panel,
        staging_state.focus == StagingFocus::Diff,
    );

    // Zone de message commit.
    render_commit_input(
        frame,
        &staging_state.commit_message,
        staging_state.cursor_position,
        staging_state.focus == StagingFocus::CommitMessage,
        !staging_state.staged_files.is_empty(),
        layout.commit_message,
    );

    // Help bar.
    render_staging_help(frame, &staging_state.focus, layout.help_bar);
}
```

#### Rendu de la liste de fichiers

```rust
fn render_file_list(
    frame: &mut Frame,
    title: &str,
    files: &[StatusEntry],
    selected: usize,
    is_focused: bool,
    area: Rect,
) {
    let items: Vec<ListItem> = files.iter().map(|entry| {
        let status_icon = match entry.display_status() {
            s if s.contains("staged") => "●",
            "Modifié" => "M",
            "Supprimé" => "D",
            "Non suivi" => "?",
            _ => " ",
        };
        // ... construire la ligne avec icône + couleur + path
    }).collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let count = files.len();
    let list = List::new(items)
        .block(Block::default()
            .title(format!(" {} ({}) ", title, count))
            .borders(Borders::ALL)
            .border_style(border_style))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}
```

#### Rendu du champ de saisie du message

```rust
fn render_commit_input(
    frame: &mut Frame,
    message: &str,
    cursor_pos: usize,
    is_focused: bool,
    has_staged_files: bool,
    area: Rect,
) {
    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let title = if has_staged_files {
        " Message de commit (Enter pour valider) "
    } else {
        " Message de commit (aucun fichier staged) "
    };

    let display_text = if message.is_empty() && !is_focused {
        "Appuyez sur 'c' pour écrire un message de commit..."
    } else {
        message
    };

    let paragraph = Paragraph::new(display_text)
        .block(Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style));

    frame.render_widget(paragraph, area);

    // Si focalisé, positionner le curseur.
    if is_focused {
        frame.set_cursor_position((
            area.x + cursor_pos as u16 + 1,
            area.y + 1,
        ));
    }
}
```

---

### 3.8 — Keybindings de la vue Staging

**Fichier** : `src/ui/input.rs`

Quand `view_mode == ViewMode::Staging` :

```rust
fn map_staging_key(key: KeyEvent, staging: &StagingState) -> Option<AppAction> {
    // Ctrl+C quitte toujours.
    // ...

    match staging.focus {
        StagingFocus::Unstaged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('s') | KeyCode::Enter => Some(AppAction::StageFile),
            KeyCode::Char('a') => Some(AppAction::StageAll),
            KeyCode::Tab => Some(AppAction::SwitchStagingFocus),  // → Staged
            KeyCode::Char('c') => Some(AppAction::StartCommitMessage),
            KeyCode::Char('1') => Some(AppAction::SwitchToGraph),
            KeyCode::Char('3') => Some(AppAction::SwitchToBranches),
            _ => None,
        },
        StagingFocus::Staged => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),
            KeyCode::Char('u') | KeyCode::Enter => Some(AppAction::UnstageFile),
            KeyCode::Char('U') => Some(AppAction::UnstageAll),
            KeyCode::Tab => Some(AppAction::SwitchStagingFocus),  // → Diff
            KeyCode::Char('c') => Some(AppAction::StartCommitMessage),
            _ => None,
        },
        StagingFocus::Diff => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(AppAction::MoveDown),  // scroll diff
            KeyCode::Char('k') | KeyCode::Up => Some(AppAction::MoveUp),      // scroll diff
            KeyCode::Tab => Some(AppAction::SwitchStagingFocus),  // → Unstaged
            KeyCode::Esc => Some(AppAction::SwitchStagingFocus),
            _ => None,
        },
        StagingFocus::CommitMessage => match key.code {
            KeyCode::Enter => Some(AppAction::ConfirmCommit),
            KeyCode::Esc => Some(AppAction::CancelCommitMessage),
            KeyCode::Char(c) => Some(AppAction::InsertChar(c)),
            KeyCode::Backspace => Some(AppAction::DeleteChar),
            KeyCode::Left => Some(AppAction::MoveCursorLeft),
            KeyCode::Right => Some(AppAction::MoveCursorRight),
            _ => None,
        },
    }
}
```

---

### 3.9 — Intégrer dans la boucle de rendu

**Fichier** : `src/ui/mod.rs`

Ajouter le dispatch vers la vue staging :

```rust
pub fn render(frame: &mut Frame, app: &App) {
    match app.view_mode {
        ViewMode::Graph => {
            // Rendu existant...
        }
        ViewMode::Staging => {
            staging_view::render(
                frame,
                &app.staging_state,
                &app.current_branch,
                &app.repo_path,
                app.flash_message.as_ref().map(|(msg, _)| msg.as_str()),
            );
        }
        ViewMode::Help => { /* ... */ }
        ViewMode::Branches => { /* STEP 4 */ }
    }
}
```

**Note** : Cela implique de refactorer la signature de `render()` pour accepter `&App`
directement au lieu de tous les champs individuels. C'est une bonne opportunité de
simplifier.

---

### 3.10 — Rafraîchissement automatique

Quand on revient de la vue Staging vers la vue Graph (touche `1`), il faut
appeler `self.refresh()` pour que le graphe reflète les nouveaux commits.

---

## Fichiers impactés

| Fichier | Modifications |
|---------|--------------|
| `src/app.rs` | Nouveaux types (`StagingFocus`, `StagingState`), nouvelles actions, logique de staging |
| `src/ui/staging_view.rs` | **Nouveau fichier** — Vue complète de staging |
| `src/ui/staging_layout.rs` | **Nouveau fichier** — Layout de la vue staging |
| `src/ui/input.rs` | Keybindings contextuel pour la vue staging, saisie de texte |
| `src/ui/mod.rs` | Ajouter modules + dispatch du rendu par `ViewMode` |
| `src/git/commit.rs` | Ajouter `unstage_file()`, `unstage_all()` |
| `src/git/repo.rs` | Méthodes `is_staged()` / `is_unstaged()` sur `StatusEntry` |
| `src/git/diff.rs` | Réutilisation de `working_dir_file_diff()` du STEP 2 |

---

## Critères de validation

- [ ] La touche `2` bascule vers la vue Staging depuis n'importe quelle vue
- [ ] La touche `1` revient à la vue Graph
- [ ] Les fichiers unstaged et staged sont dans deux panneaux séparés
- [ ] On peut naviguer dans chaque panneau avec j/k
- [ ] Le diff du fichier survolé s'affiche en temps réel à droite
- [ ] Stage un fichier (`s` ou `Enter` dans unstaged) le déplace vers staged
- [ ] Unstage un fichier (`u` ou `Enter` dans staged) le déplace vers unstaged
- [ ] `a` stage tous les fichiers d'un coup
- [ ] `c` ouvre le champ de saisie du message de commit
- [ ] `Enter` dans le champ de message crée le commit si des fichiers sont staged
- [ ] `Esc` annule la saisie du message
- [ ] Un message flash confirme la création du commit
- [ ] Le Tab cycle entre les panneaux (Unstaged → Staged → Diff → Unstaged)
- [ ] `cargo clippy` ne génère aucun warning
