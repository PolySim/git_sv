# STEP-02 - Reparer la saisie dans la vue Branches

## Type
Bug fonctionnel

## Priorite
Haute

## Constat
La vue Branches ouvre bien un mode Input dans `src/handler/branch.rs`, mais `src/handler/edit.rs` ne modifie que `staging_state.commit_message`. Les frappes mappees depuis `src/ui/input.rs` pour creer/renommer une branche, creer un stash avec message ou preparer un worktree n'alimentent donc pas `branches_view_state.input_text`.

Resultat probable : l'overlay d'input s'affiche, mais l'utilisateur ne peut pas saisir le texte attendu.

## Objectif
Rendre la saisie de texte pleinement fonctionnelle dans la vue Branches, avec edition, curseur et validation coherents.

## Etapes a suivre
1. Lister tous les contextes de saisie branches concernes : `CreateBranch`, `RenameBranch`, `SaveStash`, `CreateWorktree` dans `src/state/view/branches.rs`.
2. Choisir une strategie claire pour l'edition de texte :
   - soit generaliser `EditHandler` pour qu'il sache cibler plusieurs buffers ;
   - soit introduire un mini-editeur reutilisable partage entre Staging et Branches.
3. Ajouter une notion explicite de "buffer actif" afin que `EditAction` s'applique au bon champ selon le contexte.
4. Couvrir toutes les operations d'edition minimales dans le buffer Branches : insertion, suppression avant/apres curseur, deplacement gauche/droite, home/end et eventuellement collage.
5. Verifier la coherence entre `focus == BranchesFocus::Input`, `input_action` et le buffer affiche dans `src/ui/branches_view.rs`.
6. Gerer proprement la validation : `Enter` doit envoyer le texte courant au bon handler et `Esc` doit annuler sans effet de bord.
7. Definir un comportement clair pour les champs vides :
   - annulation silencieuse ;
   - ou message explicite selon l'action.
8. Factoriser la logique d'edition commune avec le message de commit pour eviter deux implementations qui divergent.
9. Ajouter des tests unitaires et d'integration sur les flux suivants :
   - creer une branche ;
   - renommer une branche ;
   - saisir un message de stash ;
   - annuler une saisie.

## Validation attendue
- Le texte tape apparait dans l'overlay de la vue Branches.
- Le curseur suit correctement les editions.
- `Enter` et `Esc` ont un comportement stable et previsible.

## Risques / points d'attention
- Le mapping clavier de `src/ui/input.rs` doit rester compatible avec la saisie de chiffres et caracteres speciaux.
- Il faut eviter de casser la saisie existante du message de commit.
