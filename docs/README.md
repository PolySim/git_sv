# 📋 Plan d'Amélioration de git_sv

Ce document présente le plan de refactoring et d'amélioration du projet git_sv, structuré en 8 étapes progressives.

---

## 🎯 Vue d'ensemble

| Étape | Description | Priorité | Effort | Risque |
|-------|-------------|----------|--------|--------|
| **STEP 01** | Corrections immédiates (clippy, bugs) | 🔴 Haute | 1-2h | Faible |
| **STEP 02** | Système d'erreurs et code mort | 🔴 Haute | 2-3h | Faible |
| **STEP 03** | Utilitaires communs UI | 🔴 Haute | 3-4h | Moyen |
| **STEP 04** | Refactoring de state.rs | 🔴 Haute | 4-6h | Élevé |
| **STEP 05** | Split de event.rs | 🔴 Haute | 6-8h | Élevé |
| **STEP 06** | Performances (DiffCache) | 🟡 Moyenne | 2-3h | Faible |
| **STEP 07** | Tests | 🟡 Moyenne | 4-6h | Faible |
| **STEP 08** | Nouvelles fonctionnalités | 🟢 Basse | Variable | Variable |

**Effort total estimé**: ~25-35 heures de développement

---

## 🔍 Problèmes identifiés

### Code Quality
- ⚠️ 11 warnings Clippy (imports inutilisés, variables mortes)
- ⚠️ Bugs potentiels avec troncature UTF-8
- ⚠️ Code mort (champs non lus, variant jamais construit)

### Architecture
- 🔴 `event.rs`: 3400+ lignes, 98 handlers
- 🔴 `state.rs`: 600 lignes, 17 types mélangés
- 🔴 `AppAction`: enum avec 100+ variants
- 🟡 Duplication UI: `centered_rect()` copié 5 fois

### Performance
- 🟡 DiffCache O(n) sur chaque accès
- 🟡 Rechargement complet du graph à chaque modification

### Tests
- 🔴 0% de couverture sur les handlers
- 🔴 0% de couverture sur l'UI
- 🟢 ~60% sur le module git

---

## 📁 Structure des fichiers STEP

```
docs/
├── ARCHITECTURE.md                    # Documentation existante
├── README.md                          # Ce fichier
├── STEP_01_CORRECTIONS_IMMEDIATES.md  # Bugs, clippy, imports
├── STEP_02_ERREURS_ET_CODE_MORT.md    # Système d'erreurs
├── STEP_03_UTILITAIRES_COMMUNS_UI.md  # Composants UI réutilisables
├── STEP_04_REFACTORING_STATE.md       # ListSelection, AppAction split
├── STEP_05_SPLIT_EVENT_RS.md          # Handlers modulaires
├── STEP_06_PERFORMANCES.md            # DiffCache LRU, optimisations
├── STEP_07_TESTS.md                   # Tests unitaires et intégration
└── STEP_08_NOUVELLES_FONCTIONNALITES.md # Features futures
```

---

## 🚀 Ordre d'implémentation recommandé

### Phase 1: Stabilisation (1-2 jours)
1. **STEP 01** - Corrections immédiates
   - Supprimer imports/variables inutilisés
   - Fixer les bugs de troncature UTF-8
   
2. **STEP 02** - Erreurs et code mort
   - Enrichir `GitSvError`
   - Nettoyer le code mort

### Phase 2: Fondations (3-5 jours)
3. **STEP 03** - Utilitaires UI
   - Créer `src/ui/common/`
   - Éliminer les duplications

4. **STEP 04** - Refactoring state
   - Créer `ListSelection<T>`
   - Diviser `AppAction` en sous-enums

### Phase 3: Refactoring majeur (5-7 jours)
5. **STEP 05** - Split event.rs
   - Créer `src/handler/`
   - Migrer les 98 handlers

6. **STEP 06** - Performances
   - Implémenter DiffCache avec crate `lru`
   - Optimisations mineures

### Phase 4: Qualité (ongoing)
7. **STEP 07** - Tests
   - Tests des handlers
   - Tests UI (snapshots)
   - Couverture cible: 60%

### Phase 5: Évolution (ongoing)
8. **STEP 08** - Nouvelles features
   - Filtrage du graph
   - Diff side-by-side
   - Rebase interactif
   - Et plus...

---

## ✅ Checklist globale

Avant de commencer:
```bash
# État actuel
cargo build          # ✓ Compile
cargo test           # ✓ 51 tests passent
cargo clippy         # ⚠️ 11 warnings
```

Après STEP 01-05:
```bash
cargo clippy -- -D warnings  # Doit passer sans warning
cargo test                    # Tous les tests passent
cargo run                     # Fonctionnel
```

Après STEP 06-07:
```bash
cargo tarpaulin --out Html   # Couverture > 60%
```

---

## 📊 Métriques de succès

| Métrique | Avant | Après |
|----------|-------|-------|
| Warnings Clippy | 11 | 0 |
| Plus gros fichier (lignes) | 3400 | ~400 |
| Duplications `centered_rect` | 5 | 1 |
| Couverture tests | ~30% | 60%+ |
| Variants `AppAction` | 100+ | ~15 (avec délégation) |
| Fichiers handler | 1 | 15 |

---

## 🔗 Liens utiles

- [AGENTS.md](../AGENTS.md) - Guidelines pour les agents IA
- [Cargo.toml](../Cargo.toml) - Dépendances
- [README.md](../README.md) - Documentation utilisateur

---

## 📝 Notes

- Chaque STEP est **indépendant** mais suit un ordre logique
- Les STEPs 01-03 peuvent être faits en parallèle
- Le STEP 05 est le plus risqué, prévoir du temps de test
- Le STEP 08 est une liste de suggestions, pas un plan strict
