# STEP-06 - Implementer le filtre par chemin du graphe

## Type
Fonctionnalite manquante

## Priorite
Haute

## Constat
Le popup de filtre expose un champ `path` dans `src/state/filter.rs`, mais `GraphFilter::matches()` n'applique pas ce critere. Le commentaire indique explicitement que l'information manque dans `CommitInfo`.

Resultat : l'interface laisse penser que le filtre par chemin fonctionne alors qu'il est probablement inoperant.

## Objectif
Rendre le filtre par chemin reel, performant et comprehensible pour l'utilisateur.

## Etapes a suivre
1. Valider le comportement produit cible : filtrer les commits qui touchent un chemin exact, un prefixe ou un motif partiel.
2. Choisir l'endroit ou enrichir l'information :
   - etendre `CommitInfo` avec les chemins modifies ;
   - ou appliquer le filtre dans `GitRepo::build_graph_filtered()` via une lecture du diff/trees.
3. Evaluer le cout de performance du calcul sur un historique large et definir une strategie de limitation/cache.
4. Decider de la semantique du filtre :
   - sensible ou non a la casse ;
   - dossier vs fichier ;
   - renommage ;
   - chemins relatifs au repo.
5. Faire remonter clairement dans la documentation UI ce qui est supporte exactement.
6. Ajouter un message visuel quand un filtre est actif et qu'il retourne zero resultat.
7. Verifier que la combinaison avec les autres filtres (auteur, date, message) reste intuitive.
8. Ajouter des tests sur un repo temporaire avec commits touchant plusieurs fichiers et dossiers.
9. Revoir la valeur de `fetch_count = max_count * 3` si le nouveau filtre devient plus selectif sur de gros historiques.

## Validation attendue
- Un filtre par chemin modifie reellement la liste des commits affiches.
- Les tests couvrent fichier, dossier et absence de resultat.
- Le champ `path` n'induit plus l'utilisateur en erreur.

## Risques / points d'attention
- Le filtrage par chemin peut vite devenir couteux ; il faut mesurer et eventuellement mettre en cache.
- Les renommages de fichiers doivent etre explicitement acceptes ou exclus.
