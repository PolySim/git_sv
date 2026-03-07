# STEP-07 - Corriger la selection des branches locales et distantes

## Type
Bug fonctionnel + UX

## Priorite
Haute

## Constat
`BranchesViewState::selected_branch()` dans `src/state/view/branches.rs` retourne directement une branche distante des que `show_remote == true`. En parallele, `src/ui/branches_view.rs` affiche une liste mixte avec sections Local puis Remote et calcule un index visuel specifique.

Resultat probable : des actions comme checkout, rename, delete ou merge peuvent viser la mauvaise branche, ou rendre les branches locales impossibles a cibler quand l'affichage remote est active.

## Objectif
Rendre la selection explicite, stable et conforme a ce que l'utilisateur voit a l'ecran.

## Etapes a suivre
1. Documenter le modele de selection cible pour la liste mixte :
   - une selection logique unique ;
   - une projection visuelle ;
   - une distinction claire entre element selectionne et sections decoratives.
2. Supprimer la logique implicite basee sur `show_remote` comme source de verite pour savoir quelle branche est selectionnee.
3. Introduire une representation explicite du choix courant, par exemple :
   - `SelectedBranch::Local(index)` ;
   - `SelectedBranch::Remote(index)`.
4. Refaire le calcul d'index visuel dans `src/ui/branches_view.rs` a partir de cette representation au lieu de deduire l'origine a posteriori.
5. Revoir les actions disponibles selon le type de branche selectionne :
   - checkout autorise ou non ;
   - rename/delete limites aux locales ;
   - merge autorise selon la strategie voulue.
6. Afficher visuellement les actions indisponibles pour une branche distante si besoin.
7. Ajouter des tests sur :
   - liste avec locales seulement ;
   - liste avec locales + remotes ;
   - bascule du toggle remote sans perte de selection incoherente.
8. Verifier la coherence du panneau detail et du copier-coller dans `handle_copy_to_clipboard`.

## Validation attendue
- L'element surligne est toujours celui utilise par l'action declenchee.
- Le toggle remote ne fait pas sauter la selection sur un autre type de branche sans raison.
- Le detail affiche les bonnes informations locale/distante.

## Risques / points d'attention
- Les listes avec en-tetes et lignes vides sont les plus fragiles ; la selection logique doit rester independante du rendu.
