# STEP-008 — Refonte complète : Gestion des conflits

## Problème

La vue de résolution de conflits actuelle (vue 4) est trop simpliste. Elle affiche les fichiers en conflit et permet de choisir ours/theirs/both par section, mais il manque :

1. La distinction entre fichiers valides (non en conflit) et fichiers en conflit
2. Une vue 3 panneaux pour les fichiers en conflit (ours / theirs / résultat)
3. La possibilité de résoudre ligne par ligne ou bloc par bloc (pas seulement section entière)
4. Une prévisualisation du résultat final avant validation
5. La validation du merge une fois tous les conflits résolus

## Fichiers concernés

- `src/git/conflict.rs` — Structures et logique de résolution
- `src/ui/conflicts_view.rs` — Rendu de la vue conflits
- `src/ui/input.rs` — `map_conflicts_key()` (l352-372)
- `src/state.rs` — `ConflictsState` (l365-389), `AppAction` (actions conflits)
- `src/event.rs` — Handlers des actions conflits

## Conception détaillée

### Layout cible

```
┌─────────────────────────────────────────────────────────────────┐
│ repo · main · Merge de 'feature/x' · 2 fichier(s) non résolu  │
├──────────────┬──────────────────────────────────────────────────┤
│ Fichiers     │  ┌─ Ours (HEAD) ──┬── Theirs ────────────────┐  │
│              │  │ line 1         │ line 1                    │  │
│ ✓ clean.rs   │  │ line 2         │ line 2 (modifié)          │  │
│ ✗ app.rs     │  │ >> conflit <<  │ >> conflit <<              │  │
│ ✗ lib.rs     │  │ line 4         │ line 4                    │  │
│              │  └────────────────┴───────────────────────────┘  │
│              │  ┌─ Résultat ─────────────────────────────────┐  │
│              │  │ line 1                                     │  │
│              │  │ line 2 (choix appliqué)                    │  │
│              │  │ line 3 (résolu)                            │  │
│              │  │ line 4                                     │  │
│              │  └────────────────────────────────────────────┘  │
├──────────────┴──────────────────────────────────────────────────┤
│ Mode: bloc  o:ours t:theirs b:both  l:ligne  B:bloc  F:fichier │
└─────────────────────────────────────────────────────────────────┘
```

### Panneaux

1. **Liste des fichiers** (gauche, ~25%) : Tous les fichiers du merge
   - `✓` fichiers propres (pas en conflit) en vert
   - `✗` fichiers en conflit en rouge
   - `◉` fichier en conflit résolu en jaune

2. **Vue Ours / Theirs** (droite haut, 2 panneaux cote à cote, ~50% de la hauteur)
   - Panneau gauche : version "ours" (HEAD) avec lignes en conflit surlignées
   - Panneau droit : version "theirs" (branche mergée) avec lignes en conflit surlignées
   - Les lignes en conflit sont surlignées et numérotées
   - Navigation synchronisée entre les deux panneaux

3. **Vue Résultat** (droite bas, ~50% de la hauteur)
   - Prévisualisation du fichier résolu en temps réel
   - Mise à jour automatique quand on choisit ours/theirs/both
   - Les zones non encore résolues sont marquées avec un placeholder

### Modes de résolution

Trois granularités de résolution, switchables avec des touches :

- **`F`** — Fichier entier : Résoudre tout le fichier avec ours ou theirs
- **`B`** — Bloc par bloc (mode par défaut) : Résoudre chaque section de conflit indépendamment (comportement actuel amélioré)
- **`L`** — Ligne par ligne : Sélectionner individuellement chaque ligne dans les sections de conflit

### Structures de données modifiées

```rust
/// Mode de résolution des conflits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictResolutionMode {
    File,   // Résolution par fichier entier
    Block,  // Résolution par bloc/section (défaut)
    Line,   // Résolution ligne par ligne
}

/// Résolution par ligne dans une section.
#[derive(Debug, Clone, PartialEq)]
pub struct LineResolution {
    /// Index de la ligne dans la section.
    pub line_index: usize,
    /// Source choisie pour cette ligne.
    pub source: ConflictResolution,
}

/// Section de conflit enrichie.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictSection {
    pub context_before: Vec<String>,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
    pub context_after: Vec<String>,
    /// Résolution par bloc (None si non résolu).
    pub resolution: Option<ConflictResolution>,
    /// Résolutions par ligne (vide si mode bloc).
    pub line_resolutions: Vec<LineResolution>,
}

/// État de la vue conflits (refonte).
pub struct ConflictsState {
    /// Tous les fichiers du merge (en conflit ou non).
    pub all_files: Vec<MergeFile>,
    /// Index du fichier sélectionné.
    pub file_selected: usize,
    /// Index de la section de conflit sélectionnée.
    pub section_selected: usize,
    /// Index de la ligne sélectionnée (mode ligne).
    pub line_selected: usize,
    /// Mode de résolution actif.
    pub resolution_mode: ConflictResolutionMode,
    /// Scroll dans le panneau ours.
    pub ours_scroll: usize,
    /// Scroll dans le panneau theirs.
    pub theirs_scroll: usize,
    /// Scroll dans le panneau résultat.
    pub result_scroll: usize,
    /// Focus dans les panneaux (Ours / Theirs / Result).
    pub panel_focus: ConflictPanelFocus,
    /// Description de l'opération en cours.
    pub operation_description: String,
}

/// Fichier dans un merge (en conflit ou non).
#[derive(Debug, Clone)]
pub struct MergeFile {
    pub path: String,
    pub has_conflicts: bool,
    pub conflicts: Vec<ConflictSection>,
    pub is_resolved: bool,
}

/// Focus dans les panneaux de la vue conflits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictPanelFocus {
    FileList,
    OursPanel,
    TheirsPanel,
    ResultPanel,
}
```

### Nouvelles actions

```rust
pub enum AppAction {
    // ... existants ...
    /// Changer le mode de résolution (File/Block/Line).
    ConflictSwitchMode,
    /// En mode ligne : sélectionner la ligne suivante.
    ConflictLineDown,
    /// En mode ligne : sélectionner la ligne précédente.
    ConflictLineUp,
    /// Basculer le focus entre les panneaux (Ours/Theirs/Result).
    ConflictSwitchPanel,
    /// Valider le merge final (tous les conflits résolus).
    ConflictValidateMerge,
}
```

### Keybindings révisés

| Touche | Action |
|--------|--------|
| `j/k` | Naviguer dans les sections/lignes (selon le mode) |
| `Tab` | Changer de panneau (fichiers → ours → theirs → résultat) |
| `Shift+Tab` | Panneau précédent |
| `o` | Choisir "ours" pour la sélection actuelle |
| `t` | Choisir "theirs" pour la sélection actuelle |
| `b` | Choisir "both" pour la sélection actuelle |
| `F` | Mode fichier entier |
| `B` | Mode bloc (défaut) |
| `L` | Mode ligne par ligne |
| `Enter` | Valider la résolution du fichier courant |
| `V` | Valider le merge (quand tout est résolu) |
| `q/Esc` | Annuler le merge |

### Flux de validation

1. L'utilisateur résout tous les conflits de chaque fichier
2. Quand un fichier est entièrement résolu, il passe de `✗` à `◉` 
3. Quand tous les fichiers sont résolus, `V` devient disponible
4. `V` affiche une confirmation "Finaliser le merge ? (y/n)"
5. Après confirmation, le commit de merge est créé et on retourne à la vue Graph

## Tests

- Créer un merge avec 3 fichiers : 1 propre, 2 en conflit
- Résoudre en mode bloc : vérifier que le résultat est correct
- Résoudre en mode ligne : vérifier que chaque ligne est indépendante
- Résoudre en mode fichier : vérifier que tout le fichier est remplacé
- Finaliser le merge et vérifier le commit
- Annuler le merge et vérifier que l'état est propre
- Vérifier la synchronisation du scroll entre les panneaux ours/theirs
