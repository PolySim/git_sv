# Ordre de priorité des étapes

## Bugs (priorité haute)

| #   | Étape                           | Fichier                         | Complexité | Dépendances |
| --- | ------------------------------- | ------------------------------- | ---------- | ----------- |
| 1   | Bug SSH Pull/Fetch              | `STEP_01_BUG_PULL_SSH.md`       | Moyenne    | Aucune      |
| 2   | Bug saisie clavier (lettre `r`) | `STEP_02_BUG_KEYBOARD_INPUT.md` | Faible     | Aucune      |

## Features (priorité moyenne à basse)

| #   | Étape                      | Fichier                               | Complexité | Dépendances      |
| --- | -------------------------- | ------------------------------------- | ---------- | ---------------- |
| 3   | Améliorer le Push          | `STEP_03_FEAT_PUSH.md`                | Faible     | STEP_01          |
| 4   | Améliorer l'UX Merge       | `STEP_04_FEAT_MERGE_UX.md`            | Moyenne    | Aucune           |
| 5   | Vue résolution de conflits | `STEP_05_FEAT_CONFLICT_RESOLUTION.md` | Élevée     | STEP_01, STEP_04 |

## Ordre d'implémentation recommandé

```
STEP_01 (Bug SSH) ──────────┐
                             ├──→ STEP_03 (Push)
STEP_02 (Bug clavier) ──────┘

STEP_04 (Merge UX) ─────────┐
                             ├──→ STEP_05 (Conflits)
STEP_01 (Bug SSH) ───────────┘
```

**Chemin critique** : STEP_01 → STEP_02 → STEP_03 → STEP_04 → STEP_05

Les STEP 01 et 02 peuvent être faites en parallèle (aucune dépendance entre elles).
