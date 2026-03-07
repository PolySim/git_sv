# STEP-13 - Ajouter des tests d'integration sur les flux UI critiques

## Type
Qualite / prevention de regressions

## Priorite
Haute

## Constat
Le projet a deja une bonne base de tests, mais beaucoup de risques identifies concernent le cablage entre `input -> dispatcher -> handler -> state`. Les regressions les plus probables ne sont pas dans les primitives Git elles-memes, mais dans les transitions d'etat de l'application.

## Objectif
Couvrir les parcours utilisateur a plus forte valeur et a plus fort risque de regression.

## Etapes a suivre
1. Identifier une liste courte de parcours critiques a verrouiller en priorite :
   - saisie de branche/stash/worktree ;
   - confirmations destructives ;
   - bascule Graph -> fichiers -> diff ;
   - pull avec conflits ;
   - selection branches locales/distantes ;
   - filtres et recherche.
2. Mettre en place des helpers de test pour simuler des suites d'`AppAction` ou de `KeyEvent` sans lancer toute la TUI.
3. Uniformiser la creation d'un `AppState` de test avec repo temporaire, historique et fichiers modifies realistes.
4. Ajouter des assertions de haut niveau sur :
   - le `ViewMode` ;
   - le focus ;
   - les selections ;
   - les messages flash ;
   - les confirmations actives.
5. Prioriser les tests d'integration sur les zones ou plusieurs modules cooperent, pas seulement les petits handlers isoles.
6. Completer avec quelques tests de rendu/UX legers si certaines aides dependent de l'etat.
7. Ajouter au besoin des snapshots limites pour des ecrans critiques, sans rendre la suite trop fragile.
8. Integrer ces tests dans la routine standard `cargo test`.

## Validation attendue
- Les regressions de cablage clavier et d'etat sont detectees automatiquement.
- Les bugs deja identifies dans l'audit obtiennent chacun au moins un test de non-regression.
- Les contributeurs peuvent refactorer l'etat et les handlers avec plus de securite.

## Risques / points d'attention
- Les tests ne doivent pas dependre d'un rendu terminal fragile si ce n'est pas necessaire.
- Il faut viser les parcours metier critiques avant d'augmenter la volumetrie.
