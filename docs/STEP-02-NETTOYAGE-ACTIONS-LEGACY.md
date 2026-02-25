# STEP 02 — Nettoyage du système d'actions legacy

**Priorité** : Haute
**Effort estimé** : 2-3 jours
**Impact** : Maintenabilité, réduction de la complexité du code

---

## Constat

Le fichier `src/state/action/mod.rs` contient un enum `AppAction` avec **~180 variantes**. La moitié sont des variantes "legacy" (flat) qui dupliquent les variantes structurées (sub-enums). Le code contient des commentaires explicites :

> `// TODO: Migrer vers les sous-enums et supprimer ces variantes`

Le dispatcher (`src/handler/dispatcher.rs`, 729 lignes) route les deux formats vers les mêmes handlers, doublant la surface de maintenance.

---

## Actions à mener

### 2.1 — Inventaire des variantes legacy vs structurées

Faire un mapping exhaustif entre les variantes legacy et leur équivalent structuré :

| Legacy (flat) | Structuré (sub-enum) |
|---|---|
| `AppAction::MoveUp` | `AppAction::Navigation(NavigationAction::MoveUp)` |
| `AppAction::MoveDown` | `AppAction::Navigation(NavigationAction::MoveDown)` |
| ... | ... |

### 2.2 — Migrer `src/ui/input.rs` vers les variantes structurées

Le fichier `input.rs` (647 lignes) est le principal producteur d'actions. Remplacer toutes les émissions de variantes legacy par les variantes structurées.

### 2.3 — Migrer les autres émetteurs d'actions

Rechercher tous les endroits qui créent des `AppAction::*` legacy et les migrer.

### 2.4 — Supprimer les variantes legacy du enum

Une fois tous les émetteurs migrés, supprimer les variantes legacy de `AppAction`.

### 2.5 — Simplifier le dispatcher

Retirer les branches de matching legacy dans `dispatcher.rs`. Le fichier devrait passer de ~729 lignes à ~350 lignes.

### 2.6 — Nettoyer `AppState`

Le struct `AppState` contient des champs en doublon marqués avec des commentaires du type "migrer vers graph_view.rows". Finaliser cette migration et supprimer les champs obsolètes.

---

## Stratégie de migration

Procéder module par module pour éviter de tout casser d'un coup :

1. Migrer `input.rs` (le plus gros émetteur)
2. Migrer les handlers qui émettent des actions secondaires
3. Supprimer les variantes legacy
4. Nettoyer le dispatcher

À chaque étape, exécuter `cargo test` pour vérifier l'absence de régression.

---

## Critères de validation

- [ ] Plus aucune variante legacy dans `AppAction`
- [ ] `dispatcher.rs` ne contient plus de branches de routing legacy
- [ ] `AppState` n'a plus de champs en doublon
- [ ] `cargo test` passe
- [ ] `cargo clippy` ne signale aucun warning
