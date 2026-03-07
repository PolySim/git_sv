# STEP-03 - Finaliser les confirmations d'actions destructives

## Type
Bug fonctionnel

## Priorite
Haute

## Constat
Le dialogue de confirmation sait representer `CherryPick` et `WorktreeRemove` dans `src/ui/confirm_dialog.rs`, mais `src/handler/dispatcher.rs` ne traite pas ces variantes dans `handle_confirm_action`.

Resultat probable : l'utilisateur peut confirmer visuellement l'action, mais rien ne se passe apres validation.

## Objectif
Faire en sorte que chaque confirmation affichee corresponde a une action executee, annulee ou explicitement geree.

## Etapes a suivre
1. Dresser la liste exhaustive des variantes `ConfirmAction` dans `src/ui/confirm_dialog.rs`.
2. Verifier pour chacune :
   - l'endroit ou elle est creee ;
   - l'endroit ou elle est executee ;
   - le message flash attendu ;
   - le refresh necessaire ensuite.
3. Completer `handle_confirm_action` pour couvrir au minimum `CherryPick` et `WorktreeRemove`.
4. Definir le service metier exact appele pour chaque action :
   - cherry-pick d'un commit selectionne ;
   - suppression d'un worktree cible.
5. Prevoir le traitement des erreurs metier avec messages clairs cote UI.
6. Verifier que l'annulation remet bien `pending_confirmation` a `None` sans effet secondaire.
7. Ajouter un test de non-regression par variante de confirmation importante.
8. Ajouter un garde-fou : si une variante de confirmation n'est pas implementee, renvoyer un message explicite plutot qu'un no-op silencieux.
9. Harmoniser les messages de succes/erreur pour tous les cas destructifs.

## Validation attendue
- Chaque popup de confirmation aboutit soit a une action reelle, soit a une annulation claire.
- Aucune confirmation ne se ferme sans effet si l'utilisateur repond `y`.
- Les actions qui modifient le repo marquent correctement l'etat en dirty.

## Risques / points d'attention
- Les actions Git destructives doivent etre testees sur des repos temporaires realistes.
- Pour les worktrees, il faut clarifier si la suppression vise seulement l'entree Git ou aussi le dossier associe.
