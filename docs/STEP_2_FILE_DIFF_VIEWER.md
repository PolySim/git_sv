# STEP 2 — Affichage des diffs de fichiers

## Objectif

Permettre de voir le contenu des modifications (diff ligne par ligne) d'un fichier
sélectionné dans un commit. Actuellement on voit la liste des fichiers modifiés avec
leurs stats (+/-), mais pas le contenu réel des changements.

---

## État actuel

- `src/git/diff.rs` : Calcule les `DiffFile` (path, status, additions, deletions)
  mais ne retourne pas le contenu des hunks.
- `src/ui/files_view.rs` : Affiche la liste des fichiers sans navigation ni sélection.
- `src/ui/detail_view.rs` : Affiche les métadonnées du commit (hash, auteur, date, message).

---

## Plan d'implémentation

### 2.1 — Étendre le module diff pour récupérer le contenu (`src/git/diff.rs`)

#### Nouvelles structures

```rust
/// Ligne d'un diff avec son type (ajout, suppression, contexte).
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineType {
    /// Ligne de contexte (inchangée).
    Context,
    /// Ligne ajoutée.
    Addition,
    /// Ligne supprimée.
    Deletion,
    /// En-tête de hunk (ex: @@ -10,5 +10,7 @@).
    HunkHeader,
}

/// Ligne individuelle d'un diff.
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// Type de la ligne.
    pub line_type: DiffLineType,
    /// Contenu textuel de la ligne.
    pub content: String,
    /// Numéro de ligne dans l'ancien fichier (si applicable).
    pub old_lineno: Option<u32>,
    /// Numéro de ligne dans le nouveau fichier (si applicable).
    pub new_lineno: Option<u32>,
}

/// Diff complet d'un fichier dans un commit.
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// Chemin du fichier.
    pub path: String,
    /// Statut (Added, Modified, Deleted, Renamed).
    pub status: DiffStatus,
    /// Lignes du diff.
    pub lines: Vec<DiffLine>,
    /// Nombre total d'ajouts.
    pub additions: usize,
    /// Nombre total de suppressions.
    pub deletions: usize,
}
```

#### Nouvelle fonction

```rust
/// Récupère le diff détaillé d'un fichier spécifique dans un commit.
pub fn file_diff(repo: &Repository, oid: Oid, file_path: &str) -> Result<FileDiff> {
    // 1. Obtenir le diff du commit (tree parent vs tree commit).
    // 2. Itérer sur les deltas pour trouver le fichier correspondant.
    // 3. Extraire le Patch pour ce delta.
    // 4. Itérer sur les hunks et lignes du patch.
    // 5. Construire les DiffLine avec types et numéros de lignes.
}
```

#### Détail de l'extraction des lignes

Utiliser `git2::Patch::from_diff()` puis itérer :

```rust
for hunk_idx in 0..patch.num_hunks() {
    let (hunk, _) = patch.hunk(hunk_idx)?;
    // Ajouter le header du hunk.
    lines.push(DiffLine {
        line_type: DiffLineType::HunkHeader,
        content: format!("@@ -{},{} +{},{} @@",
            hunk.old_start(), hunk.old_lines(),
            hunk.new_start(), hunk.new_lines()),
        old_lineno: None,
        new_lineno: None,
    });

    let num_lines = patch.num_lines_in_hunk(hunk_idx)?;
    for line_idx in 0..num_lines {
        let line = patch.line_in_hunk(hunk_idx, line_idx)?;
        let line_type = match line.origin() {
            '+' => DiffLineType::Addition,
            '-' => DiffLineType::Deletion,
            ' ' => DiffLineType::Context,
            _ => continue,
        };
        lines.push(DiffLine {
            line_type,
            content: String::from_utf8_lossy(line.content()).to_string(),
            old_lineno: line.old_lineno(),
            new_lineno: line.new_lineno(),
        });
    }
}
```

#### Fonction pour le diff du working directory

```rust
/// Récupère le diff d'un fichier du working directory (non committé).
pub fn working_dir_file_diff(repo: &Repository, file_path: &str) -> Result<FileDiff> {
    // Diff entre l'index (HEAD) et le working tree pour un fichier spécifique.
}
```

---

### 2.2 — Ajouter la navigation dans la liste des fichiers

**Fichier** : `src/app.rs`

#### Nouveaux champs dans `App`

```rust
pub struct App {
    // ... champs existants ...

    /// Index du fichier sélectionné dans le panneau de fichiers.
    pub file_selected_index: usize,
    /// Diff du fichier sélectionné (chargé à la demande).
    pub selected_file_diff: Option<FileDiff>,
}
```

#### Nouvelles actions

```rust
pub enum AppAction {
    // ... actions existantes ...

    /// Naviguer vers le haut dans le panneau de fichiers.
    FileUp,
    /// Naviguer vers le bas dans le panneau de fichiers.
    FileDown,
}
```

#### Logique de mise à jour

Quand le focus est sur le panneau `Files` et que l'utilisateur navigue (j/k) :
1. Mettre à jour `file_selected_index`.
2. Charger le diff du fichier sélectionné via `file_diff()`.
3. Stocker le résultat dans `selected_file_diff`.

```rust
AppAction::FileUp | AppAction::FileDown => {
    if self.focus == FocusPanel::Files {
        // Mettre à jour file_selected_index.
        // Charger le diff du fichier.
        self.load_selected_file_diff();
    }
}
```

---

### 2.3 — Remplacer le panneau Détail par le Diff Viewer

**Fichier** : `src/ui/detail_view.rs` → Transformer en panneau contextuel.

Le panneau bas-droit affichera :
- **Quand focus = Graph** : Les métadonnées du commit (comportement actuel).
- **Quand focus = Files** : Le diff du fichier sélectionné.

#### Nouveau composant : `diff_view.rs`

**Nouveau fichier** : `src/ui/diff_view.rs`

```rust
/// Rend le diff d'un fichier avec coloration syntaxique.
pub fn render(
    frame: &mut Frame,
    diff: Option<&FileDiff>,
    scroll_offset: usize,
    area: Rect,
    is_focused: bool,
) {
    // Si pas de diff, afficher un message par défaut.
    // Sinon, construire les lignes colorées.
}
```

#### Rendu des lignes de diff

```rust
fn build_diff_lines(diff: &FileDiff) -> Vec<Line<'static>> {
    diff.lines.iter().map(|line| {
        let (prefix, color, bg) = match line.line_type {
            DiffLineType::Addition => ("+", Color::Green, Some(Color::Rgb(0, 40, 0))),
            DiffLineType::Deletion => ("-", Color::Red, Some(Color::Rgb(40, 0, 0))),
            DiffLineType::Context => (" ", Color::White, None),
            DiffLineType::HunkHeader => ("", Color::Cyan, None),
        };

        let mut spans = Vec::new();

        // Numéros de lignes.
        let old_no = line.old_lineno.map(|n| format!("{:4}", n)).unwrap_or("    ".into());
        let new_no = line.new_lineno.map(|n| format!("{:4}", n)).unwrap_or("    ".into());
        spans.push(Span::styled(format!("{} {} ", old_no, new_no), Style::default().fg(Color::DarkGray)));

        // Préfixe et contenu.
        let style = Style::default().fg(color);
        let style = if let Some(bg) = bg { style.bg(bg) } else { style };
        spans.push(Span::styled(format!("{}{}", prefix, line.content), style));

        Line::from(spans)
    }).collect()
}
```

#### Scroll dans le diff

Ajouter un `diff_scroll_offset: usize` dans `App` pour permettre de scroller
verticalement dans le diff quand le fichier est long.

Touches :
- `j/k` quand focus = Detail : scroller le diff
- `Ctrl+d/u` : page down/up dans le diff

---

### 2.4 — Adapter le panneau bas-droit pour être contextuel

**Fichier** : `src/ui/mod.rs`

Le panneau bas-droit (`bottom_right`) doit maintenant être contextuel :

```rust
// Rendu du panneau bas-droit.
match focus {
    FocusPanel::Graph | FocusPanel::Detail => {
        // Afficher les métadonnées du commit (comme avant).
        detail_view::render(frame, graph, selected_index, layout.bottom_right, is_detail_focused);
    }
    FocusPanel::Files => {
        // Afficher le diff du fichier sélectionné.
        diff_view::render(frame, selected_file_diff.as_ref(), diff_scroll, layout.bottom_right, false);
    }
}
```

---

### 2.5 — Mettre à jour les keybindings

**Fichier** : `src/ui/input.rs`

Quand le focus est sur `Files` :
- `j/k` : naviguer dans la liste des fichiers (au lieu des commits)
- `Enter` : basculer le focus sur le diff (panneau Detail)

Quand le focus est sur `Detail` et qu'un diff est affiché :
- `j/k` : scroller le diff
- `Esc` : revenir au panneau Files

---

### 2.6 — Mettre à jour le `files_view.rs` pour la sélection

**Fichier** : `src/ui/files_view.rs`

Ajouter un highlight sur le fichier sélectionné :

```rust
pub fn render(
    frame: &mut Frame,
    commit_files: &[DiffFile],
    // ...
    file_selected_index: usize,  // NOUVEAU
    // ...
) {
    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut state = ListState::default();
    state.select(Some(file_selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}
```

---

## Fichiers impactés

| Fichier | Modifications |
|---------|--------------|
| `src/git/diff.rs` | Nouveaux types (`DiffLine`, `DiffLineType`, `FileDiff`), nouvelles fonctions (`file_diff`, `working_dir_file_diff`) |
| `src/ui/diff_view.rs` | **Nouveau fichier** — Rendu du diff avec coloration |
| `src/ui/mod.rs` | Ajouter `pub mod diff_view`, adapter le render contextuel |
| `src/ui/files_view.rs` | Ajouter sélection avec highlight |
| `src/ui/input.rs` | Keybindings contextuels selon le focus |
| `src/app.rs` | Nouveaux champs (`file_selected_index`, `selected_file_diff`, `diff_scroll_offset`), nouvelles actions |
| `src/git/repo.rs` | Nouvelle méthode `file_diff()` |

---

## Critères de validation

- [ ] On peut naviguer dans la liste des fichiers d'un commit avec j/k
- [ ] Le fichier sélectionné est visuellement mis en évidence
- [ ] Le diff du fichier sélectionné s'affiche dans le panneau droit
- [ ] Les ajouts sont en vert, les suppressions en rouge, le contexte en blanc
- [ ] Les numéros de lignes sont affichés
- [ ] Les headers de hunk (`@@...@@`) sont visibles en cyan
- [ ] On peut scroller dans un diff long
- [ ] Le diff du working directory fonctionne aussi (mode WorkingDir)
- [ ] `cargo clippy` ne génère aucun warning
