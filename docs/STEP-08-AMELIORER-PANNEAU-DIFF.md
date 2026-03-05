# STEP 08 — Améliorer le panneau diff du file panel

**Priorité** : Haute
**Effort estimé** : Moyen
**Impact** : Expérience utilisateur critique — le diff est actuellement quasi illisible

---

## Constat

Quand on est sur le panneau fichiers (BottomLeft) et qu'on sélectionne un fichier, le diff s'affiche dans le panneau BottomRight. Deux problèmes majeurs :

1. **Le panneau est trop petit** : Le layout actuel découpe 60% graph / 40% bottom, puis 50/50 entre fichiers et diff. Le diff n'occupe donc que ~20% de l'écran — insuffisant pour lire du code.
2. **Le scroll est limité** : Le scroll existe (`ScrollDiffUp`/`ScrollDiffDown`) mais uniquement ligne par ligne. Pas de page-up/page-down, pas de scrollbar visuelle, pas de borne max (on peut scroller dans le vide), et pas de scroll horizontal.

### Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/ui/layout.rs` | Calcul du layout (ligne 51, `build_layout()`) |
| `src/ui/diff_view.rs` | Rendu du diff (ligne 18, `render()`) |
| `src/ui/mod.rs` | Choix entre detail_view et diff_view (lignes 206-227) |
| `src/ui/input.rs` | Keybindings scroll (lignes 242-244, 188-196, 388-393) |
| `src/handler/navigation.rs` | Handlers scroll (`handle_scroll_diff_down` ligne 190, `handle_scroll_diff_up` ligne 184) |
| `src/state/mod.rs` | État : `diff_scroll_offset: usize` (ligne 90) |
| `src/state/view/graph.rs` | État dupliqué : `diff_scroll_offset` (ligne 16) |

---

## Actions à mener

### 8.1 — Mode diff plein écran (priorité haute)

Ajouter un mode "zoom" pour le diff qui utilise toute la zone bottom (ou même tout l'écran) :

**Proposition** : Quand le focus est sur BottomRight (diff visible), appuyer sur `z` ou `Enter` bascule en mode plein écran pour le diff.

- Ajouter un état `diff_fullscreen: bool` dans `AppState`
- Dans `layout.rs`, quand `diff_fullscreen` est actif :
  - Soit utiliser toute la zone `main_content` (graph masqué)
  - Soit utiliser toute la zone `bottom` (100% au lieu de 50%)
- Ajouter la touche `z` (ou `Enter`) dans les keybindings BottomRight pour toggler
- Ajouter `Esc` pour quitter le mode plein écran
- Afficher un indicateur visuel `[ZOOM]` ou `[PLEIN ÉCRAN]` dans le titre du panneau diff

### 8.2 — Améliorer le scroll vertical (priorité haute)

Le scroll actuel incrémente de 1 sans borne. Améliorer :

- **Page up/down** : Ajouter `Ctrl+d`/`Ctrl+u` (ou `Page Up`/`Page Down`) pour scroller d'une demi-page quand le focus est sur BottomRight. Note : `Ctrl+d`/`Ctrl+u` sont déjà mappés mais seulement pour le scroll ligne par ligne — les faire scroller de `area.height / 2` lignes.
- **Borne maximale** : Calculer le nombre total de lignes du diff et empêcher `diff_scroll_offset` de dépasser `total_lines.saturating_sub(visible_height)`.
- **Scroll rapide** : `g` / `G` pour aller au début/fin du diff.

**Modifications** :
- `handler/navigation.rs` : Modifier `handle_scroll_diff_down()` (ligne 190) pour accepter un `amount: usize` paramètre, ajouter une borne max. Modifier `handle_scroll_diff_up()` (ligne 184) de même.
- `state/mod.rs` : Ajouter `diff_total_lines: usize` pour mémoriser la taille du diff chargé.
- `ui/input.rs` : Ajouter les variantes `ScrollDiffPageDown`, `ScrollDiffPageUp`, `ScrollDiffTop`, `ScrollDiffBottom` dans `NavigationAction` ou utiliser les existants avec un paramètre.

### 8.3 — Ajouter un scroll horizontal (priorité moyenne)

Les diffs de fichiers avec des lignes longues sont tronqués. Ajouter :

- Un état `diff_horizontal_offset: usize` dans `AppState`
- Les touches `h`/`l` (ou `←`/`→`) pour scroller horizontalement quand le focus est sur BottomRight
- Appliquer `.scroll((vertical_offset, horizontal_offset))` sur le `Paragraph` dans `diff_view.rs` (actuellement le 2ème paramètre est toujours `0`, lignes 68 et 143/148)

### 8.4 — Indicateur de position dans le scroll (priorité basse)

Afficher une indication de la position actuelle dans le diff :

- Option 1 : Afficher `[ligne X/Y]` dans le titre du bloc diff
- Option 2 : Utiliser le widget `Scrollbar` de ratatui en position droite du panneau
- Mettre à jour à chaque scroll

### 8.5 — Synchroniser l'état dupliqué (priorité basse)

Il y a actuellement une duplication entre `AppState.diff_scroll_offset` (ligne 90) et `GraphViewState.diff_scroll_offset` (ligne 16) qui ne sont pas toujours synchronisés. Décider d'une source unique de vérité et migrer les deux usages vers `graph_view.diff_scroll_offset`.

---

## Critères de validation

- [ ] Le mode plein écran du diff est fonctionnel (`z` pour toggler)
- [ ] Page up/down fonctionne dans le diff
- [ ] Le scroll ne dépasse pas les bornes du diff
- [ ] Le scroll horizontal fonctionne
- [ ] Un indicateur de position est visible
- [ ] `cargo clippy` propre
- [ ] `cargo test` passe
