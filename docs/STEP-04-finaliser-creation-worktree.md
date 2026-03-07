# STEP-04 - Finaliser la creation de worktree

## Type
Fonctionnalite inachevee

## Priorite
Haute

## Constat
La vue Branches expose une action de creation de worktree (`n` dans l'onglet Worktrees), mais `handle_worktree_create` dans `src/handler/branch.rs` est vide. Le type `InputAction::CreateWorktree` existe, ainsi que le titre d'overlay associe dans `src/ui/branches_view.rs`, mais le flux n'est pas cable de bout en bout.

## Objectif
Rendre la creation de worktree utilisable, comprehensible et sure depuis la TUI.

## Etapes a suivre
1. Definir le parcours utilisateur cible :
   - point d'entree depuis l'onglet Worktrees ;
   - champs necessaires ;
   - validations avant creation ;
   - message de succes et rafraichissement.
2. Remplacer le format libre actuel `nom chemin [branche]` par une experience plus robuste :
   - soit plusieurs champs ;
   - soit un parseur strict avec aide contextuelle visible.
3. Implementer `handle_worktree_create` pour ouvrir une saisie reelle avec `InputAction::CreateWorktree`.
4. Valider les entrees avant execution :
   - chemin non vide ;
   - repertoire cible inexistant ou vide ;
   - nom unique ;
   - branche source explicite ou strategie par defaut documentee.
5. Verifier les cas limites : branche inexistante, dossier deja present, HEAD detachee, worktree dupliqe.
6. Apres creation, recharger la liste des worktrees et reselectionner si possible le worktree cree.
7. Afficher dans le detail du worktree la branche attachee et le statut principal/secondaire de maniere fiable.
8. Ajouter des tests sur un repo temporaire avec verification du chemin, du nom et de la branche associee.
9. Mettre a jour l'aide contextuelle de la vue Branches et le README si le parcours change.

## Validation attendue
- Depuis l'onglet Worktrees, `n` ouvre une saisie exploitable.
- La creation reussie apparait immediatement dans la liste.
- Les erreurs d'entree ou de creation sont explicites.

## Risques / points d'attention
- Le format libre actuel est fragile ; mieux vaut une UX plus guidee qu'un parseur permissif.
- Il faut distinguer creation d'un worktree sur branche existante et creation avec nouvelle branche.
