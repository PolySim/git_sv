# STEP-05 - Rendre le pull background compatible avec les conflits

## Type
Bug fonctionnel + UX

## Priorite
Haute

## Constat
La boucle principale dans `src/handler/mod.rs` intercepte `Push`, `Pull` et `Fetch` pour les envoyer en arriere-plan. Pour `Pull`, le thread appelle `pull_current_branch_cli_path` dans `src/git/remote.rs` et ne remonte qu'un `String` de succes/erreur.

En parallele, `src/handler/git.rs` contient deja une logique plus riche via `pull_current_branch_with_result` qui sait ouvrir la vue conflits. Les deux chemins ne se comportent donc pas pareil.

## Objectif
Conserver les avantages du background loading sans perdre les transitions metier necessaires quand un pull genere des conflits.

## Etapes a suivre
1. Cartographier les deux parcours existants :
   - parcours synchrone typé ;
   - parcours background par CLI.
2. Definir une structure de resultat background plus riche que `Result<String, String>` pour remonter :
   - succes simple ;
   - fast-forward ;
   - deja a jour ;
   - conflits ;
   - erreur.
3. Choisir une strategie d'implementation :
   - rendre le worker capable de detecter l'etat post-pull ;
   - ou faire le fetch/pull en background puis interpreter le resultat sur le thread principal.
4. Preserver le spinner et l'interaction non bloquante pendant l'operation.
5. En cas de conflit, construire correctement `ConflictsState` et basculer en `ViewMode::Conflicts` comme le chemin synchrone.
6. Unifier les messages flash entre les deux parcours pour eviter des retours differents selon le mode d'execution.
7. Ajouter des tests de regression sur :
   - repo deja a jour ;
   - fast-forward ;
   - merge automatique ;
   - conflit de pull.
8. Revoir si `Push` et `Fetch` ont aussi besoin de resultats plus riches a moyen terme.

## Validation attendue
- Un pull qui genere des conflits ouvre la vue Conflits meme quand il passe par le thread background.
- Les messages de succes distinguent `deja a jour`, `fast-forward` et `merge`.
- Le spinner disparait toujours proprement a la fin de l'operation.

## Risques / points d'attention
- `git2::Repository` n'etant pas `Send`, il faut bien separer ce qui est execute dans le thread et ce qui est reconstruit ensuite dans le thread principal.
- Le comportement CLI et libgit2 doit rester coherent pour ne pas surprendre l'utilisateur.
