# STEP-10 - Ajouter un chargement progressif de l'historique

## Type
Fonctionnalite manquante + performance UX

## Priorite
Moyenne a haute

## Constat
`MAX_COMMITS` vaut 200 dans `src/state/mod.rs`. Cette limite fixe est simple, mais elle devient vite bloquante sur des repos reels : recherche incomplete, graphe tronque, impossibilite de naviguer au-dela d'un historique recent.

## Objectif
Permettre d'explorer des historiques volumineux sans penaliser le demarrage ni surcharger le rendu.

## Etapes a suivre
1. Definir le comportement cible : pagination manuelle, chargement infini, ou "charger plus" via un raccourci dedie.
2. Choisir une representation d'etat pour distinguer :
   - nombre actuellement charge ;
   - nombre total estime ;
   - chargement en cours ou non.
3. Adapter `GitRepo::log_all_branches()` et la construction du graphe pour accepter des fenetres ou curseurs plus progressifs.
4. Faire remonter visuellement dans la TUI que l'historique est partiel et qu'il peut etre etendu.
5. Preserver la selection courante lors du chargement de commits supplementaires.
6. Verifier l'interaction avec la recherche et les filtres :
   - recherche sur la fenetre chargee seulement ;
   - ou extension automatique du chargement.
7. Evaluer l'impact sur le temps de rendu et la memoire, surtout dans `src/git/graph.rs`.
8. Ajouter des tests de non-regression sur le maintien de la selection et la taille du graphe apres extension.
9. Documenter clairement la limite initiale et le comportement de chargement dans l'aide utilisateur.

## Validation attendue
- Le graphe ne reste plus bloque a 200 commits sans option utilisateur.
- Le chargement initial reste rapide.
- La navigation reste fluide meme apres extension de l'historique.

## Risques / points d'attention
- Le calcul du graphe peut devenir couteux ; il faudra peut-etre combiner pagination et cache.
- Les index de selection et de rendu sont sensibles aux ajouts de lignes de connexion.
