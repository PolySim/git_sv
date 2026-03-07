# STEP-08 - Aligner les raccourcis, l'aide et les comportements reels

## Type
Bug UX / coherence produit

## Priorite
Haute

## Constat
Plusieurs sources se contredisent :
- `README.md` inverse `push` et `pull` par rapport a `src/ui/input.rs` ;
- `src/ui/help_overlay.rs` annonce `Enter` pour le diff plein ecran alors que le comportement reel depend du focus ;
- l'aide de la vue Branches annonce `Ctrl+P` alors que ce raccourci n'est pas traite dans les mappings de cette vue ;
- la touche `b` ouvre une fonctionnalite marquee comme stub dans `src/handler/git.rs` alors qu'un autre mode Branches existe deja.

## Objectif
Avoir une seule verite produit pour les keybindings, les overlays d'aide et les comportements exposes.

## Etapes a suivre
1. Faire un inventaire complet des keybindings reels dans `src/ui/input.rs` par vue.
2. Comparer cet inventaire avec :
   - `README.md` ;
   - `src/ui/help_overlay.rs` ;
   - `src/ui/help_bar.rs` ;
   - `src/ui/branches_view.rs` ;
   - `docs/ARCHITECTURE.md`.
3. Corriger les divergences les plus trompeuses en priorite : push/pull, `Enter`, `Ctrl+P`, `b`.
4. Decider du futur du panneau `b` :
   - le finaliser ;
   - ou le retirer au profit de la vue Branches complete.
5. Centraliser la definition des keybindings dans une source unique ou un schema commun pour limiter la derive documentaire.
6. Generer ou composer les aides contextuelles a partir de cette source unique plutot que maintenir plusieurs textes manuels.
7. Verifier que l'aide contextuelle change bien selon le focus reel et pas seulement selon la vue.
8. Ajouter un test leger sur quelques raccourcis critiques pour verrouiller la coherence de mapping.
9. Mettre a jour le README avec des captures ou exemples de parcours reels si necessaire.

## Validation attendue
- La documentation utilisateur decrit exactement ce que fait l'application.
- Les raccourcis critiques sont coherents dans toutes les vues.
- L'utilisateur n'est plus incite a utiliser une fonctionnalite stub ou non supportee.

## Risques / points d'attention
- Sans source unique, la derive reviendra vite. Il faut traiter la cause et pas seulement les symptomes.
