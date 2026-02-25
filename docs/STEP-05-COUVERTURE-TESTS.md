# STEP 05 — Amélioration de la couverture de tests

**Priorité** : Haute
**Effort estimé** : 3-5 jours
**Impact** : Confiance dans les refactorings, détection des régressions

---

## Constat

Le projet a ~120 tests unitaires, ce qui est un bon début. Cependant :

- **Handlers non testés** : 7 handlers sur 10 n'ont aucun test
- **Tests d'intégration** : seuls 3 tests basiques existent
- **UI** : seuls 3 modules UI sur 20+ ont des tests
- Le README mentionne "35 tests" alors qu'il y en a ~120 (information obsolète)

---

## Actions à mener

### 5.1 — Tester les handlers manquants

Handlers sans tests (critique) :

| Handler | Priorité | Complexité |
|---|---|---|
| `handler/dispatcher.rs` | Haute | Le routing est critique, tester chaque variante d'action |
| `handler/conflict.rs` | Haute | 1024 lignes de logique complexe sans aucun test |
| `handler/branch.rs` | Haute | Checkout, create, delete — opérations destructives |
| `handler/git.rs` | Haute | Push, pull, fetch — opérations réseau |
| `handler/search.rs` | Moyenne | Logique simple mais à couvrir |
| `handler/edit.rs` | Moyenne | Édition de texte, curseur |
| `handler/filter.rs` | Moyenne | Filtres sur le graphe |

Utiliser le `TestStateBuilder` existant dans `src/test_utils/test_state.rs` et les assertions custom dans `src/test_utils/assertions.rs`.

### 5.2 — Enrichir les tests d'intégration

Le dossier `tests/` contient 3 tests d'intégration très basiques. Ajouter des scénarios :

- **Workflow complet** : clone → edit → stage → commit → push
- **Branches** : create → checkout → merge → delete
- **Conflits** : créer un conflit → résoudre → commit
- **Stash** : stash → checkout → stash pop
- **Filtrage** : filtrer par auteur, date, message
- **Recherche** : rechercher par message, auteur, hash

### 5.3 — Ajouter des tests UI avec snapshots

Le projet a déjà `insta` en dev-dependency et un helper `render_to_string` dans `src/ui/tests/mod.rs`. Étendre les snapshot tests à :

- `staging_view.rs` — rendu avec/sans fichiers staged
- `branches_view.rs` — rendu des 3 onglets
- `conflicts_view.rs` — rendu des 3 panneaux
- `blame_view.rs` — rendu du blame
- `filter_popup.rs` — rendu du popup
- `confirm_dialog.rs` — rendu de la confirmation

### 5.4 — Tester `git/conflict.rs`

Le fichier le plus complexe du projet (1213 lignes) avec :
- Parsing de marqueurs de conflit
- Résolution ours/theirs/both
- Édition ligne par ligne

Ajouter des tests pour chaque stratégie de résolution avec des fixtures de fichiers conflictuels.

### 5.5 — Tester `git/remote.rs`

Les opérations réseau (push, pull, fetch) sont délicates à tester. Options :

- Tester avec un bare repo local comme remote
- Mocker les callbacks de credentials
- Tester la gestion d'erreurs (remote inexistant, authentification échouée)

### 5.6 — Mesurer et suivre la couverture

Configurer `cargo tarpaulin` (déjà dans `Cargo.toml`) en CI :

```bash
cargo tarpaulin --out Html --output-dir coverage
```

Définir un objectif de couverture : **70% minimum** comme première cible, puis **80%**.

---

## Critères de validation

- [ ] Tous les handlers ont au moins des tests pour les cas principaux
- [ ] Tests d'intégration couvrent les workflows complets
- [ ] Snapshot tests pour les composants UI principaux
- [ ] `git/conflict.rs` a des tests pour chaque stratégie de résolution
- [ ] Couverture mesurée et > 70%
- [ ] Nombre réel de tests documenté dans le README
