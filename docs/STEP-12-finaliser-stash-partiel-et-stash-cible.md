# STEP-12 - Finaliser le stash partiel et le stash cible

## Type
Fonctionnalite manquante / stub expose a l'utilisateur

## Priorite
Moyenne

## Constat
Dans `src/handler/staging.rs`, les actions `StashSelectedFile` et `StashUnstagedFiles` existent mais sont explicitement marquees comme stubs. Pourtant des raccourcis sont deja exposes dans la vue Staging (`S`, `Ctrl+S`).

## Objectif
Supprimer l'ecart entre ce que l'interface promet et ce que le produit sait faire pour les workflows de stash fins.

## Etapes a suivre
1. Decider si ces raccourcis doivent etre :
   - implementes reellement ;
   - masques tant qu'ils ne sont pas disponibles.
2. Si la fonctionnalite est gardee, definir le perimetre exact :
   - stash du fichier selectionne ;
   - stash des changements non stagues uniquement ;
   - message de stash optionnel ou automatique.
3. Choisir l'approche technique la plus robuste : appel CLI Git cible ou orchestration libgit2 + index.
4. Ajouter les confirmations ou messages necessaires pour expliquer l'effet exact sur l'index et le working tree.
5. Recharger l'etat de Staging apres operation avec mise a jour du diff visible.
6. Gerer les cas limites : fichier supprime, fichier renomme, combinaison staged/unstaged sur le meme chemin.
7. Ajouter des tests sur des repos temporaires avec melange de modifications staguees et non staguees.
8. Mettre a jour l'aide de la vue Staging pour n'afficher que des raccourcis reels.

## Validation attendue
- Les raccourcis de stash exposes ont un effet concret, documente et testable.
- L'utilisateur comprend clairement ce qui a ete mis de cote.
- L'etat de staging est coherent immediatement apres l'action.

## Risques / points d'attention
- Les workflows de stash partiel sont parmi les plus sensibles cote Git ; il faut des tests de bout en bout solides.
