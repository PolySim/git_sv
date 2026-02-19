# STEP-010 — UX : Désactiver le panneau détail en vue Graph + diff visuel en vue Staging

## Problème (2 sous-problèmes)

### 10a — Panneau détail inutile en vue Graph

En vue Graph (vue 1), le panneau "Détail" (en bas à droite) n'apporte aucune information que le graphe ne donne pas déjà. Le graphe affiche déjà le hash, l'auteur, la date, le message, et les refs. Le Tab cycle entre Graph → Files → Detail, ce qui ajoute une étape de navigation inutile.

### 10b — Diff purement visuel en vue Staging

En vue Staging (vue 2), le panneau diff à droite est purement visuel (lecture seule). L'utilisateur ne peut pas interagir avec (pas de stage/unstage par ligne par exemple). Ce n'est pas un bug mais une limitation à noter.

## Fichiers concernés

### 10a
- `src/state.rs` — `FocusPanel` enum (l186-190) : contient `Detail`
- `src/ui/input.rs` — `map_key()` : Tab cycle vers Detail
- `src/ui/layout.rs` — Layout du graph (split bas en 2 : Files | Detail)
- `src/ui/detail_view.rs` — Rendu du panneau détail
- `src/ui/mod.rs` — Dispatch du rendu
- `src/event.rs` — Handler de `SwitchBottomMode`

### 10b
- `src/ui/staging_view.rs` — Panneau diff
- `src/ui/diff_view.rs` — Rendu du diff

## Solution 10a — Supprimer le panneau Detail du cycle de focus

### Option A — Supprimer le panneau Detail entièrement

Retirer `FocusPanel::Detail` et remplacer le layout du bas par un seul panneau Files + Diff intégré (quand un fichier est sélectionné, le diff s'affiche en dessous ou à droite des fichiers).

### Option B — Garder le panneau mais retirer du cycle Tab (recommandé)

Garder le panneau Detail visible (il affiche le diff du fichier sélectionné) mais retirer `Detail` du cycle Tab. Le Tab alterne uniquement entre `Graph` et `Files` :

```rust
// Dans event.rs, handler de SwitchBottomMode
AppAction::SwitchBottomMode => {
    match state.focus {
        FocusPanel::Graph => {
            state.focus = FocusPanel::Files;
            // Auto-sélectionner le premier fichier (cf. STEP-003)
        }
        FocusPanel::Files => {
            state.focus = FocusPanel::Graph;
        }
        FocusPanel::Detail => {
            // Ne devrait plus arriver, mais fallback
            state.focus = FocusPanel::Graph;
        }
    }
}
```

Le panneau droit (ex-Detail) affiche automatiquement le diff du fichier sélectionné dans Files, sans nécessiter de focus dessus. Le scroll du diff se fait avec les touches dédiées (`Ctrl+d`/`Ctrl+u`) quand le focus est sur Files.

### Option C — Remplacer Detail par un panneau Diff interactif

Transformer le panneau en diff interactif : quand un fichier est sélectionné dans Files, le panneau droit montre le diff. Les keybindings `j/k` dans Files naviguent dans les fichiers, et `Enter` ou `→` permet de "rentrer" dans le diff pour le scroller.

### Recommandation

Option B est la plus simple et la moins disruptive. Le panneau reste visible et utile (il montre le diff du fichier sélectionné) mais ne nécessite plus de navigation explicite.

## Solution 10b — Note pour amélioration future

Le diff en vue Staging est actuellement en lecture seule. Pour une version future, on pourrait ajouter :
- Stage/unstage par hunk (`s` sur un hunk)
- Stage/unstage par ligne (`s` sur une ligne)
- Similaire au comportement de `git add -p`

Ce travail est hors scope de ce STEP et pourrait faire l'objet d'un STEP séparé.

## Tests

- Vérifier que Tab alterne uniquement entre Graph et Files (pas de Detail)
- Vérifier que le diff du fichier sélectionné s'affiche bien dans le panneau droit
- Vérifier que Ctrl+d/Ctrl+u scrollent le diff depuis le focus Files
- Vérifier que Esc depuis Files retourne au Graph
