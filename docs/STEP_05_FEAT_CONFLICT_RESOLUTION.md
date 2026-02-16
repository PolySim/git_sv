# STEP 05 — Feature : Vue de résolution de conflits

## Contexte

Lors d'un merge, d'un pull, d'un checkout ou d'un cherry-pick, des conflits peuvent survenir. Actuellement, git_sv affiche simplement un message d'erreur (`"Conflits détectés"`). L'objectif est de créer une 4ème vue dédiée à la résolution de conflits, similaire à ce que propose GitKraken.

## Comportement souhaité

### Déclenchement automatique

Quand une opération git produit des conflits (merge, pull, cherry-pick, rebase), la vue de résolution s'ouvre automatiquement.

### Interface à 3 panneaux (style GitKraken)

```
┌─────────────────── Résolution de conflits ───────────────────┐
│                                                               │
│  Fichiers en conflit         │  Résolution du fichier         │
│  ─────────────────           │  ─────────────────────         │
│  ● src/main.rs               │  ┌───────┐  ┌───────┐         │
│    src/app.rs                │  │ OURS  │  │THEIRS │         │
│    src/utils/mod.rs          │  │(HEAD) │  │(merge)│         │
│                              │  └───┬───┘  └───┬───┘         │
│                              │      │          │              │
│                              │      v          v              │
│                              │  ┌──────────────────┐         │
│                              │  │     RÉSULTAT     │         │
│                              │  │  (fichier final) │         │
│                              │  └──────────────────┘         │
│                                                               │
├───────────────────────────────────────────────────────────────┤
│  j/k:naviguer  o:ours  t:theirs  b:both  Enter:valider       │
│  Tab:fichier suivant  R:résoudre tout (ours/theirs)  q:abort  │
└───────────────────────────────────────────────────────────────┘
```

### Fonctionnalités

- **Liste des fichiers en conflit** (panneau gauche) : Affiche tous les fichiers ayant des marqueurs de conflit. Icône pour indiquer le statut (non résolu / résolu).
- **Vue du conflit** (panneau droit) : Pour chaque section en conflit dans le fichier, afficher les deux versions (ours = HEAD, theirs = branche mergée).
- **Choix par section** : Pour chaque bloc de conflit, l'utilisateur peut choisir :
  - `o` — Garder la version "ours" (HEAD)
  - `t` — Garder la version "theirs" (branche mergée)
  - `b` — Garder les deux (both)
  - Édition manuelle possible (optionnel, version avancée)
- **Validation** : Quand tous les conflits d'un fichier sont résolus, le fichier est automatiquement marqué comme résolu (`git add`). Quand tous les fichiers sont résolus, proposer de finaliser le merge (commit).

## Plan d'implémentation

### Étape 1 — Modèle de données pour les conflits

**`src/git/conflict.rs`** (nouveau fichier) :

```rust
/// Un fichier en conflit.
pub struct ConflictFile {
    pub path: String,
    pub conflicts: Vec<ConflictSection>,
    pub is_resolved: bool,
}

/// Une section de conflit dans un fichier.
pub struct ConflictSection {
    /// Lignes de contexte avant le conflit.
    pub context_before: Vec<String>,
    /// Version "ours" (HEAD / branche courante).
    pub ours: Vec<String>,
    /// Version "theirs" (branche mergée).
    pub theirs: Vec<String>,
    /// Lignes de contexte après le conflit.
    pub context_after: Vec<String>,
    /// Résolution choisie par l'utilisateur.
    pub resolution: Option<ConflictResolution>,
}

pub enum ConflictResolution {
    Ours,
    Theirs,
    Both,
}
```

Fonctions à implémenter :
- `parse_conflict_file(path: &str) -> Result<Vec<ConflictSection>>` : Parser les marqueurs `<<<<<<<`, `=======`, `>>>>>>>` dans un fichier.
- `list_conflict_files(repo: &Repository) -> Result<Vec<ConflictFile>>` : Lister les fichiers en conflit via `repo.index()?.conflicts()`.
- `resolve_file(repo: &Repository, path: &str, sections: &[ConflictSection]) -> Result<()>` : Écrire le fichier résolu (sans marqueurs) et faire `git add`.
- `abort_merge(repo: &Repository) -> Result<()>` : Annuler le merge en cours (`repo.cleanup_state()`).

### Étape 2 — État de la vue conflits

**`src/state.rs`** — Ajouter :

```rust
pub enum ViewMode {
    Graph,
    Help,
    Staging,
    Branches,
    Blame,
    Conflicts,  // ← Nouveau
}

pub struct ConflictsState {
    /// Liste des fichiers en conflit.
    pub files: Vec<ConflictFile>,
    /// Index du fichier sélectionné.
    pub file_selected: usize,
    /// Index de la section de conflit sélectionnée dans le fichier courant.
    pub section_selected: usize,
    /// Offset de scroll dans le panneau de résolution.
    pub scroll_offset: usize,
    /// Description de l'opération en cours (ex: "Merge de 'feature/x' dans 'main'").
    pub operation_description: String,
}
```

Ajouter `pub conflicts_state: Option<ConflictsState>` dans `AppState`.

### Étape 3 — Nouvelles actions

**`src/state.rs`** — Ajouter les actions :

```rust
pub enum AppAction {
    // ...existants...
    /// Basculer vers la vue conflits.
    SwitchToConflicts,
    /// Résoudre la section avec "ours".
    ConflictChooseOurs,
    /// Résoudre la section avec "theirs".
    ConflictChooseTheirs,
    /// Résoudre la section avec les deux.
    ConflictChooseBoth,
    /// Passer au fichier en conflit suivant.
    ConflictNextFile,
    /// Passer au fichier en conflit précédent.
    ConflictPrevFile,
    /// Passer à la section de conflit suivante.
    ConflictNextSection,
    /// Passer à la section de conflit précédente.
    ConflictPrevSection,
    /// Valider la résolution du fichier courant.
    ConflictResolveFile,
    /// Finaliser le merge (tous les conflits résolus).
    ConflictFinalize,
    /// Annuler le merge en cours.
    ConflictAbort,
}
```

### Étape 4 — Keybindings de la vue conflits

**`src/ui/input.rs`** — Ajouter `map_conflicts_key()` :

```rust
fn map_conflicts_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    match key.code {
        // Navigation entre fichiers
        KeyCode::Tab => Some(AppAction::ConflictNextFile),
        KeyCode::BackTab => Some(AppAction::ConflictPrevFile),
        // Navigation entre sections de conflit
        KeyCode::Char('j') | KeyCode::Down => Some(AppAction::ConflictNextSection),
        KeyCode::Char('k') | KeyCode::Up => Some(AppAction::ConflictPrevSection),
        // Résolution
        KeyCode::Char('o') => Some(AppAction::ConflictChooseOurs),
        KeyCode::Char('t') => Some(AppAction::ConflictChooseTheirs),
        KeyCode::Char('b') => Some(AppAction::ConflictChooseBoth),
        // Validation / Annulation
        KeyCode::Enter => Some(AppAction::ConflictResolveFile),
        KeyCode::Char('F') => Some(AppAction::ConflictFinalize),
        KeyCode::Char('q') | KeyCode::Esc => Some(AppAction::ConflictAbort),
        // Vues
        KeyCode::Char('?') => Some(AppAction::ToggleHelp),
        _ => None,
    }
}
```

Appeler cette fonction dans `map_key()` quand `state.view_mode == ViewMode::Conflicts`.

### Étape 5 — Rendu de la vue conflits

**`src/ui/conflicts_view.rs`** (nouveau fichier) :

Structure du layout :
1. **Status bar** : `"git_sv · résolution de conflits · Merge de 'feature/x' dans 'main'"`
2. **Panneau gauche** : Liste des fichiers en conflit avec icônes de statut :
   - `✗` rouge = non résolu
   - `✓` vert = résolu
3. **Panneau droit** : Affichage du conflit sélectionné :
   - Lignes de contexte (grisées)
   - Bloc "ours" (coloré en vert / bleu)
   - Séparateur `=======`
   - Bloc "theirs" (coloré en rouge / magenta)
   - Si résolu : afficher la résolution choisie avec un fond coloré
4. **Help bar** : Raccourcis contextuels

### Étape 6 — Intégration avec les opérations existantes

Modifier les fonctions qui peuvent générer des conflits pour rediriger vers la vue :

**`src/git/merge.rs`** — Quand `index.has_conflicts()` :
- Au lieu de retourner une erreur, retourner un type spécial indiquant les conflits.

**`src/event.rs`** — Points d'intégration :

1. **Merge** (`handle_confirm_input` → `InputAction::MergeBranch` ou le nouveau `MergePickerConfirm`) :
```rust
if let Err(e) = merge_branch(...) {
    if is_conflict_error(&e) {
        self.open_conflicts_view("Merge de '...' dans '...'")?;
    } else {
        self.state.set_flash_message(format!("Erreur: {}", e));
    }
}
```

2. **Pull** (`handle_git_pull`) :
```rust
// Après le merge du pull, si conflits détectés
if has_conflicts {
    self.open_conflicts_view("Pull depuis origin")?;
}
```

3. **Cherry-pick** (`execute_cherry_pick`) :
```rust
if has_conflicts {
    self.open_conflicts_view(format!("Cherry-pick de {}", commit_oid))?;
}
```

### Étape 7 — Logique de résolution

**`src/event.rs`** — Handlers :

- `handle_conflict_choose_ours()` : Marquer la section comme résolue avec `Ours`.
- `handle_conflict_choose_theirs()` : Marquer la section comme résolue avec `Theirs`.
- `handle_conflict_choose_both()` : Marquer la section comme résolue avec `Both`.
- `handle_conflict_resolve_file()` : Écrire le fichier sans marqueurs de conflit, faire `git add`.
- `handle_conflict_finalize()` : Vérifier que tous les fichiers sont résolus, créer le commit de merge.
- `handle_conflict_abort()` : Exécuter `repo.cleanup_state()` pour annuler le merge, retourner à la vue Graph.

### Étape 8 — Touche `4` pour accéder à la vue conflits

Ajouter la navigation `4` dans les keybindings globaux (seulement visible quand des conflits sont en cours) :
```rust
KeyCode::Char('4') => {
    if state.conflicts_state.is_some() {
        return Some(AppAction::SwitchToConflicts);
    }
}
```

Mettre à jour la barre de navigation pour afficher `[4] Conflits` quand un état de conflit est actif.

## Fichiers à créer / modifier

| Fichier | Action |
|---------|--------|
| `src/git/conflict.rs` | **Nouveau** — Parser et résoudre les conflits |
| `src/git/mod.rs` | Ajouter `pub mod conflict;` |
| `src/ui/conflicts_view.rs` | **Nouveau** — Rendu de la vue de résolution |
| `src/ui/conflicts_layout.rs` | **Nouveau** — Layout de la vue (panneau gauche / droit) |
| `src/ui/mod.rs` | Ajouter les nouveaux modules, rendre la vue si `ViewMode::Conflicts` |
| `src/state.rs` | Ajouter `ViewMode::Conflicts`, `ConflictsState`, nouvelles actions |
| `src/ui/input.rs` | Ajouter `map_conflicts_key()` et navigation `4` |
| `src/event.rs` | Ajouter tous les handlers de résolution, modifier les handlers de merge/pull/cherry-pick |
| `src/git/merge.rs` | Modifier pour retourner un résultat typé (conflit vs erreur) |
| `src/git/remote.rs` | Modifier `pull_current_branch()` pour détecter les conflits proprement |
| `src/ui/nav_bar.rs` | Ajouter l'onglet `[4] Conflits` conditionnel |

## Complexité

**Élevée** — C'est la feature la plus complexe du lot :
- Parsing des marqueurs de conflit
- Interface à 3 panneaux avec navigation multi-niveaux
- Écriture de fichiers résolus
- Intégration avec plusieurs opérations git existantes

## Dépendances

- **STEP_04** : Le nouveau système de merge (MergePicker) doit être en place pour que les conflits soient détectés et redirigés correctement.
- **STEP_01** : Le pull doit fonctionner pour pouvoir tester les conflits lors d'un pull.

## Priorité

**Basse à moyenne** — Feature avancée, à implémenter après les bugs critiques et les améliorations de merge.
