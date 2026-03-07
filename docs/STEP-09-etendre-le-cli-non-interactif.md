# STEP-09 - Etendre le CLI non interactif

## Type
Fonctionnalite manquante

## Priorite
Moyenne a haute

## Constat
Le CLI dans `src/main.rs` expose seulement `--path` et `log -n/--max-count`. Pour un outil positionne comme "git graph CLI/TUI", la partie non interactive est tres limitee.

## Objectif
Rendre l'outil utile aussi dans des scripts, pipelines ou usages rapides sans entrer dans la TUI.

## Etapes a suivre
1. Definir le perimetre minimal d'un vrai mode CLI non interactif :
   - `log` enrichi ;
   - `branches` ;
   - `status` ;
   - `search` ;
   - `graph` textuel ou exportable.
2. Prioriser les commandes qui reutilisent au maximum les services deja presents dans `src/git/`.
3. Definir un format de sortie coherent :
   - humain lisible par defaut ;
   - option machine (`json` ou `plain`) pour scripting.
4. Ajouter des options de filtrage utiles cote CLI : auteur, message, chemin, nombre de commits, branche cible.
5. Decider ce qui doit rester strictement TUI (conflits, edition complexe, staging interactif) et ce qui doit etre disponible en CLI.
6. Extraire la logique de presentation de `print_log()` pour qu'elle devienne reutilisable et testable.
7. Ajouter des tests de CLI sur les principales commandes et leurs sorties.
8. Mettre a jour le README avec des exemples pratiques orientes terminal pur.

## Validation attendue
- Un utilisateur peut inspecter rapidement un repo sans lancer la TUI.
- Les sorties sont stables et testables.
- Le positionnement du produit devient plus clair entre CLI et TUI.

## Risques / points d'attention
- Il faut eviter de dupliquer la logique metier entre TUI et CLI ; le partage doit se faire dans `src/git/` et des couches de presentation fines.
