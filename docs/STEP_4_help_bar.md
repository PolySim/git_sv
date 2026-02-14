# STEP 4 — Barre d'aide persistante & overlay d'aide complet

## Problème actuel

- Aucune indication visuelle des touches disponibles.
- La touche `?` est mappée sur `ToggleHelp` mais le mode `ViewMode::Help`
  n'a aucun rendu implémenté.
- Un nouvel utilisateur ne sait pas comment naviguer.

## Objectif

Deux niveaux d'aide :

1. **Footer bar** (toujours visible, 1 ligne en bas de l'écran) :
   Affiche les commandes principales selon le contexte actif.
   
2. **Overlay d'aide** (popup avec `?`) :
   Liste complète de tous les raccourcis avec descriptions.

## Fichiers à créer/modifier

### 1. Créer `src/ui/help_bar.rs` — Footer persistant

Barre de 1 ligne de haut, fond coloré (style status bar), affichant les
raccourcis contextuels :

**Contexte graphe :**
```
 j/k:naviguer  Enter:détail  b:branches  c:commit  s:stash  m:merge  Tab:fichiers  ?:aide  q:quitter
```

**Contexte panneau branches :**
```
 j/k:naviguer  Enter:checkout  n:nouvelle  d:supprimer  Esc:fermer
```

Implémentation :
```rust
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let keys = match current_context(app) {
        Context::Graph => vec![
            ("j/k", "naviguer"),
            ("Enter", "détail"),
            ("b", "branches"),
            ("c", "commit"),
            // ...
        ],
        Context::BranchPanel => vec![
            ("j/k", "naviguer"),
            ("Enter", "checkout"),
            // ...
        ],
    };
    // Formater en Spans avec couleurs alternées
}
```

Style : fond `Color::DarkGray`, touches en `Color::Cyan` + Bold,
descriptions en `Color::White`.

### 2. Créer `src/ui/help_overlay.rs` — Popup d'aide complète

Popup centré (70% largeur, 80% hauteur) avec toutes les commandes groupées :

```
┌──────────────── Aide ────────────────┐
│                                      │
│  Navigation                          │
│  ─────────                           │
│  j / ↓       Commit suivant          │
│  k / ↑       Commit précédent        │
│  g           Premier commit          │
│  G           Dernier commit          │
│  PgUp/PgDn   Page haut/bas          │
│                                      │
│  Actions Git                         │
│  ───────────                         │
│  c           Nouveau commit          │
│  s           Stash                   │
│  m           Merge                   │
│  b           Branches                │
│                                      │
│  Interface                           │
│  ─────────                           │
│  Tab         Basculer panneaux       │
│  r           Rafraîchir              │
│  ?           Aide                    │
│  q           Quitter                 │
│                                      │
│         Esc ou ? pour fermer         │
└──────────────────────────────────────┘
```

### 3. Modifier `src/ui/layout.rs` — Réserver la dernière ligne

Le layout actuel prend 100% de l'espace. Il faut réserver 1 ligne en bas
pour la help bar :

```rust
pub fn build_layout(area: Rect) -> LayoutChunks {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),       // Contenu principal
            Constraint::Length(1),    // Help bar
        ])
        .split(area);

    // ... split du contenu principal comme avant ...
    
    LayoutChunks {
        graph: ...,
        bottom_left: ...,
        bottom_right: ...,
        help_bar: outer[1],
    }
}
```

Retourner une struct `LayoutChunks` au lieu d'un `Vec<Rect>` pour plus de clarté.

### 4. Modifier `src/ui/mod.rs` — Intégrer le rendu

```rust
pub fn render(frame: &mut Frame, app: &App) {
    let layout = layout::build_layout(frame.area());

    graph_view::render(frame, app, layout.graph);
    status_view::render(frame, app, layout.bottom_left);
    detail_view::render(frame, app, layout.bottom_right);
    help_bar::render(frame, app, layout.help_bar);

    // Overlays (par-dessus tout le reste)
    if app.view_mode == ViewMode::Help {
        help_overlay::render(frame, app, frame.area());
    }
}
```

### 5. Modifier `src/app.rs` — Action ToggleHelp

L'action `ToggleHelp` existe déjà, il suffit que le rendu fonctionne.
Ajouter aussi `Esc` pour fermer le help overlay.

## Critère de validation

- La barre en bas est toujours visible et montre les touches contextuelles.
- `?` ouvre un overlay complet avec tous les raccourcis.
- `Esc` ou `?` ferme l'overlay.
- Les touches affichées changent selon le panneau actif (graphe, branches, etc.).
