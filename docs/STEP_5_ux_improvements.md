# STEP 5 — Améliorations UX / polish

## Améliorations proposées

Ce sont des améliorations que je juge pertinentes pour rendre l'application
vraiment utilisable au quotidien.

---

### 5.1 — Scroll correct avec ListState

**Problème :** Le graphe utilise un `List` simple sans `ListState`. Résultat :
pas de scroll automatique quand la sélection dépasse la zone visible.

**Solution :** Utiliser `ratatui::widgets::ListState` dans `App` :
```rust
pub graph_state: ListState,
```

Dans `MoveUp`/`MoveDown`, mettre à jour `graph_state.select(Some(index))`.
Dans le rendu, utiliser `frame.render_stateful_widget(list, area, &mut state)`.

**Fichiers :** `app.rs`, `ui/graph_view.rs`

---

### 5.2 — Status bar en haut de l'écran

**Problème :** Pas de vue d'ensemble rapide (quel repo, quelle branche, état clean/dirty).

**Solution :** Ajouter une barre de 1 ligne en haut :
```
 git_sv  main  ✓ clean               ~/Documents/mon-projet
```
ou si dirty :
```
 git_sv  main  ✗ 3 modifiés, 1 non suivi    ~/Documents/mon-projet
```

**Fichiers :** Créer `src/ui/status_bar.rs`, modifier `layout.rs` pour réserver
1 ligne en haut.

---

### 5.3 — Navigation rapide

**Problème :** Seuls `j/k` (1 par 1) sont disponibles. Lent sur un gros repo.

**Solution :** Ajouter :
- `g` / `Home` : aller au premier commit (le plus récent)
- `G` / `End` : aller au dernier commit (le plus ancien)
- `Ctrl+d` / `PgDn` : descendre d'une demi-page
- `Ctrl+u` / `PgUp` : monter d'une demi-page

**Fichiers :** `input.rs`, `app.rs` (nouvelles actions `PageUp`, `PageDown`, `GoTop`, `GoBottom`)

---

### 5.4 — Navigation entre panneaux avec Tab

**Problème :** Pas de focus clair sur quel panneau est actif.

**Solution :**
- `Tab` cycle le focus entre : Graphe -> Fichiers -> Détail -> Graphe
- Le panneau actif a une bordure plus visible (double bordure ou couleur de bordure différente)
- Les touches de navigation (j/k) s'appliquent au panneau actif

Ajouter dans `App` :
```rust
pub enum FocusPanel { Graph, Files, Detail }
pub focus: FocusPanel,
```

**Fichiers :** `app.rs`, tous les fichiers `ui/*.rs` (adapter la bordure selon le focus)

---

### 5.5 — Confirmation et messages flash

**Problème :** Les actions destructives (delete branch, drop stash) n'ont pas de
confirmation. Pas de feedback après une action (checkout réussi, commit créé).

**Solution :**
- Ajouter un champ `pub flash_message: Option<(String, Instant)>` dans `App`
- Après une action, afficher un message pendant 3 secondes dans la status bar
- Pour les actions destructives, afficher un prompt de confirmation :
  "Supprimer la branche 'feature/x' ? (y/n)"

**Fichiers :** `app.rs`, `ui/status_bar.rs`

---

### 5.6 — Revwalk multi-branches

**Problème actuel :** `repo.log()` ne parcourt que depuis HEAD. Les commits
sur des branches non mergées dans HEAD ne sont pas visibles.

**Solution :** Dans `repo.rs`, pousser toutes les branches dans le revwalk :
```rust
pub fn log_all_branches(&self, max_count: usize) -> Result<Vec<CommitInfo>> {
    let mut revwalk = self.repo.revwalk()?;
    // Pousser toutes les refs locales
    for reference in self.repo.references()? {
        let reference = reference?;
        if let Some(oid) = reference.target() {
            revwalk.push(oid).ok();
        }
    }
    revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;
    // ...
}
```

C'est essentiel pour un graphe style GitKraken : on veut voir **toutes**
les branches, pas seulement celle qui est checkout.

**Fichiers :** `git/repo.rs`, `app.rs` (appeler `log_all_branches` au lieu de `log`)

---

## Ordre de priorité recommandé

1. **5.6** Revwalk multi-branches (prérequis pour un vrai graphe)
2. **5.1** ListState scroll (utilisabilité de base)
3. **5.2** Status bar (orientation dans le repo)
4. **5.3** Navigation rapide (confort)
5. **5.4** Navigation panneaux (polish)
6. **5.5** Messages flash (feedback utilisateur)

---

## Récapitulatif des fichiers impactés

| Fichier                 | 5.1 | 5.2 | 5.3 | 5.4 | 5.5 | 5.6 |
|-------------------------|-----|-----|-----|-----|-----|-----|
| `app.rs`                |  x  |  x  |  x  |  x  |  x  |     |
| `git/repo.rs`           |     |     |     |     |     |  x  |
| `ui/graph_view.rs`      |  x  |     |     |  x  |     |     |
| `ui/layout.rs`          |     |  x  |     |     |     |     |
| `ui/status_bar.rs` (new)|     |  x  |     |     |  x  |     |
| `ui/input.rs`           |     |     |  x  |  x  |     |     |
| `ui/detail_view.rs`     |     |     |     |  x  |     |     |
| `ui/status_view.rs`     |     |     |     |  x  |     |     |
