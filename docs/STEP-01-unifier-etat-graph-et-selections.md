# STEP-01 - Unifier l'etat du graphe et des selections

## Type
Refactor clean code + prevention de bugs

## Priorite
Haute

## Constat
L'etat du graphe est duplique entre plusieurs champs dans `src/state/mod.rs` : `graph`, `graph_view.rows`, `selected_index`, `graph_state`, `commit_files`, `file_selected_index` et plusieurs methodes de synchronisation ad hoc.

Cette duplication augmente fortement le risque de desynchronisation apres un refresh, un filtrage, un changement de vue ou un chargement de diff. Le code contient deja des methodes de compatibilite (`sync_graph_selection`, `sync_legacy_selection`), ce qui indique qu'une migration est en cours mais pas terminee.

## Objectif
Avoir une seule source de verite pour :
- la liste des commits affiches ;
- la selection du commit ;
- la selection du fichier ;
- les offsets de scroll associes a la vue active.

## Etapes a suivre
1. Cartographier tous les champs legacy encore lus/ecrits dans `src/state/mod.rs`, `src/handler/navigation.rs`, `src/handler/mod.rs`, `src/ui/mod.rs`, `src/ui/graph_view.rs` et `src/ui/files_view.rs`.
2. Definir un modele cible clair : soit tout centraliser dans `GraphViewState`, soit creer un sous-etat dedie a la vue Graph qui encapsule selection, fichiers du commit, diff selectionne et scrolls.
3. Documenter les invariants a respecter dans le nouveau modele :
   - un index selectionne ne doit jamais sortir des bornes ;
   - un refresh ne doit pas casser la selection si le commit existe encore ;
   - un changement de commit doit recharger les fichiers et remettre a zero les scrolls du diff quand c'est pertinent.
4. Remplacer progressivement les lectures directes de `selected_index`, `graph_state` et `file_selected_index` par des accesseurs uniques.
5. Supprimer les methodes de synchronisation de compatibilite une fois que tous les appels passent par le nouvel etat.
6. Extraire les operations recurrentes dans des fonctions metier explicites : `select_commit`, `select_file`, `replace_graph`, `refresh_selected_commit_payload`.
7. Verifier que `App::new()` et `EventHandler::refresh()` utilisent exactement le meme chemin de mise a jour pour eviter deux logiques d'initialisation divergentes.
8. Revoir la logique `selected_index * 2` pour la projection visuelle ratatui et l'isoler dans une fonction nommee afin d'eviter les calculs copies-colles.
9. Nettoyer les champs, commentaires et alias de compatibilite qui ne servent plus.
10. Mettre a jour `docs/ARCHITECTURE.md` apres la refonte pour refleter le nouvel etat reel.

## Validation attendue
- Naviguer dans le graphe, ouvrir les fichiers, revenir en arriere et rafraichir ne provoque aucune perte de selection incoherente.
- Les filtres, la recherche et les changements de vue ne laissent jamais un index hors borne.
- Le code de `src/state/mod.rs` est significativement plus petit et sans commentaires de migration restants.

## Risques / points d'attention
- Ce chantier touche beaucoup de modules ; il faut le decouper en petites etapes.
- Les regressions les plus probables concernent la navigation, le diff plein ecran et la restauration du focus.
