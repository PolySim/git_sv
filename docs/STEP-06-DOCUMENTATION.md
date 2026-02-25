# STEP 06 — Documentation du code et du projet

**Priorité** : Moyenne
**Effort estimé** : 2-3 jours
**Impact** : Onboarding des contributeurs, maintenabilité long terme

---

## Constat

- **doc comments `///`** : Présents de manière inégale. Les fonctions publiques des modules `git/` sont plutôt bien documentées, mais beaucoup de structs, enums et fonctions dans `handler/`, `state/`, `ui/` n'ont pas de doc comments.
- **`ARCHITECTURE.md`** : Obsolète — il décrit l'architecture initiale prévue mais ne reflète plus l'état actuel (manque `handler/`, `state/`, `watcher.rs`, `error_display.rs`, la moitié des modules `git/` et `ui/`).
- **`AGENTS.md`** : Bien rédigé et à jour (instructions pour les AI agents).
- **README** : Correct mais contient des informations obsolètes (nombre de tests).

---

## Actions à mener

### 6.1 — Mettre à jour `docs/ARCHITECTURE.md`

L'architecture a considérablement évolué par rapport au document initial. Mettre à jour pour refléter :

- La structure actuelle complète (`handler/`, `state/`, `state/action/`, `state/view/`, `utils/`, `test_utils/`)
- Le flux de données réel (EventHandler → ActionDispatcher → Handlers → AppState → UI)
- Le système de cache (LRU diff cache)
- Le file watcher
- Le système de thème
- Les modes de vue (Graph, Staging, Branches, Conflicts, Blame, Help)

### 6.2 — Ajouter des doc comments aux types publics

Priorité par module :

| Module | Structs/Enums sans doc | Priorité |
|---|---|---|
| `state/mod.rs` | `AppState` (partiellement documenté) | Haute |
| `state/action/mod.rs` | `AppAction` et sous-variantes | Haute |
| `state/view/` | Tous les types d'état de vue | Moyenne |
| `handler/` | `ActionDispatcher`, `EventHandler` | Haute |
| `ui/` | Fonctions de rendu | Moyenne |
| `git/` | Types déjà bien documentés | Basse |

### 6.3 — Ajouter des doc comments modulaires `//!`

Chaque fichier `mod.rs` devrait avoir un commentaire modulaire `//!` en début de fichier expliquant le rôle du module. Actuellement manquant dans :

- `src/handler/mod.rs`
- `src/state/mod.rs`
- `src/state/action/mod.rs`
- `src/state/view/mod.rs`
- `src/ui/mod.rs`
- `src/ui/common/mod.rs`

### 6.4 — Documenter les keybindings dans le code

Le fichier `src/ui/input.rs` (647 lignes) contient toute la logique de mapping des touches, mais sans commentaires structurants. Ajouter des blocs de commentaires par mode de vue pour faciliter la navigation.

### 6.5 — Créer un CONTRIBUTING.md

Pour les contributeurs potentiels, documenter :

- Comment setup le projet de développement
- Comment lancer les tests
- Conventions de code (déjà dans AGENTS.md mais devrait être dans CONTRIBUTING.md)
- Processus de PR
- Comment ajouter une nouvelle vue ou un nouveau handler

### 6.6 — Cohérence de langue dans les doc comments

Le `AGENTS.md` dit : "Commentaires inline en français, doc comments en anglais". Cependant, de nombreux doc comments (`///`) sont en **français**. Choisir une convention et l'appliquer uniformément.

Recommandation : **tout en français** puisque c'est la langue dominante du projet et du README.

---

## Critères de validation

- [ ] `ARCHITECTURE.md` reflète l'état actuel du projet
- [ ] Tous les types publics ont des doc comments
- [ ] Tous les `mod.rs` ont un commentaire modulaire `//!`
- [ ] `CONTRIBUTING.md` créé
- [ ] Cohérence de langue dans les doc comments (tout français ou tout anglais)
- [ ] `cargo doc --no-deps` génère une documentation navigable sans warnings
