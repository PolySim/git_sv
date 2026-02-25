# STEP 03 — Élimination du code dupliqué et refactoring structurel

**Priorité** : Haute
**Effort estimé** : 2-3 jours
**Impact** : Réduction de la dette technique, maintenabilité

---

## Constat

Plusieurs patterns de code sont dupliqués à travers le projet, et certains fichiers sont trop volumineux pour être facilement maintenus.

---

## Actions à mener

### 3.1 — Dédupliquer `centered_rect()`

La fonction `centered_rect()` est **copiée dans 5 fichiers** :

- `src/ui/confirm_dialog.rs`
- `src/ui/conflicts_view.rs`
- `src/ui/filter_popup.rs`
- `src/ui/merge_picker.rs`
- `src/ui/loading.rs`

Elle existe déjà dans `src/ui/common/rect.rs`. **Supprimer les copies** et faire pointer tous les fichiers vers `ui::common::centered_rect`.

### 3.2 — Remplacer les couleurs hardcodées par le thème

Plusieurs fichiers utilisent des couleurs en dur au lieu du système de thème (`src/ui/theme.rs`) :

- `src/ui/branches_view.rs` — couleurs hardcodées pour les sections
- `src/ui/files_view.rs` — couleurs de statut en dur
- `src/ui/nav_bar.rs` — couleurs de navigation en dur

Migrer vers `AppTheme::current()` pour toutes les couleurs.

### 3.3 — Découper les fichiers trop volumineux

Fichiers critiques à découper :

| Fichier | Lignes | Suggestion |
|---|---|---|
| `git/conflict.rs` | 1,213 | Séparer parsing, résolution file-level, résolution block-level, éditeur ligne |
| `handler/conflict.rs` | 1,024 | Séparer par type de résolution (file/block/line/edit) |
| `ui/conflicts_view.rs` | 860 | Séparer le rendu des 3 panneaux (ours/theirs/result) |
| `git/graph.rs` | 853 | Séparer les types, l'algorithme de placement, et le rendering |
| `ui/graph_view.rs` | 814 | Séparer le rendu des cellules, des connexions, et la légende |
| `handler/dispatcher.rs` | 729 | Sera résolu par STEP-02 |
| `ui/input.rs` | 647 | Séparer par ViewMode (un fichier par vue) |

### 3.4 — Implémenter les handlers stub

Deux handlers sont des stubs vides :

- `handler/staging.rs` → `handle_stash_selected_file()` — ne fait rien
- `handler/staging.rs` → `handle_stash_unstaged_files()` — ne fait rien
- `handler/git.rs` → `handle_branch_list()` — ne fait rien

Soit les implémenter, soit les supprimer avec un commentaire expliquant pourquoi.

### 3.5 — Extraire les constantes magiques

Rechercher et extraire les nombres magiques dans des constantes nommées :

- `MAX_COMMITS = 200` (déjà fait dans `state/mod.rs`, bien)
- Timeout du watcher (2000ms, 500ms) → constantes nommées
- Timeout de l'event handler (100ms, 250ms) → constantes nommées
- Capacité du cache LRU (50) → constante nommée

---

## Critères de validation

- [ ] `centered_rect()` n'existe qu'à un seul endroit
- [ ] Aucune couleur hardcodée en dehors de `theme.rs`
- [ ] Aucun fichier ne dépasse 500 lignes (hors tests)
- [ ] Plus de handlers stub sans explication
- [ ] Plus de nombres magiques dans le code
- [ ] `cargo test` passe
- [ ] `cargo clippy` propre
