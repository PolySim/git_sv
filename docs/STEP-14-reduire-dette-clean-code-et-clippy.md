# STEP-14 - Reduire la dette clean code et les alertes Clippy

## Type
Refactor clean code

## Priorite
Moyenne

## Constat
`cargo clippy` remonte de nombreux avertissements, surtout dans :
- `src/git/conflict.rs` ;
- `src/git/remote.rs` ;
- `src/ui/input.rs` ;
- `src/ui/branches_view.rs` ;
- `src/ui/mod.rs` ;
- `src/handler/mod.rs`.

En plus des details stylistiques, plusieurs signaux revelent des fonctions trop longues, des branches trop complexes et des zones difficiles a maintenir.

## Objectif
Profiter de l'audit pour reduire la complexite accidentelle, clarifier les responsabilites des modules et faire de Clippy un vrai filet de securite.

## Etapes a suivre
1. Classer les avertissements Clippy par categorie :
   - hygiene simple ;
   - lisibilite ;
   - simplification de logique ;
   - fonctions trop grosses / trop d'arguments.
2. Commencer par les corrections sans risque sur les hot spots les plus frequents pour rendre le signal plus lisible.
3. Extraire les fonctions les plus lourdes :
   - mapping clavier par vue ;
   - rendu des gros panneaux ;
   - parse/merge de conflits ;
   - logique remote SSH.
4. Introduire des structures de parametres lorsque plusieurs fonctions UI depassent 7 arguments (`graph_view`, `files_view`, `diff_view`, `status_bar`, `staging_view`).
5. Factoriser les branches conditionnelles repetitives et les conversions de formatage imbriquees.
6. Nettoyer les restes de migration ou de compatibilite qui n'ont plus de valeur.
7. Traiter les noms ou enums trompeurs, par exemple `BottomLeftMode::Parents` qui represente en pratique le working tree dans `src/state/view/mod.rs`.
8. Repasser `cargo clippy --all-features -- -D warnings` comme objectif de fin de chantier.
9. Documenter les rares avertissements eventuellement conserves avec justification explicite plutot qu'avec un silence global.

## Validation attendue
- Le nombre d'avertissements Clippy chute nettement ou disparait.
- Les fonctions centrales sont plus courtes et plus explicites.
- La lecture du code des modules critiques devient plus directe.

## Risques / points d'attention
- Il faut distinguer nettoyage sans risque et refactor structurel ; ne pas tout melanger dans un seul commit.
- Les gros fichiers comme `conflict.rs` et `input.rs` meritent une decomposition progressive pour eviter les regressions.
