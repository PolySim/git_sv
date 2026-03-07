# STEP-11 - Ameliorer la navigation souris et la gestion du focus

## Type
Amelioration UI/UX

## Priorite
Moyenne

## Constat
Le terminal active la capture souris dans `src/terminal.rs`, mais `map_mouse()` dans `src/ui/input.rs` ne gere quasiment que la molette. Les clics sont ignores et la logique de focus repose surtout sur des raccourcis clavier implicites.

## Objectif
Rendre la TUI plus intuitive, notamment pour les nouveaux utilisateurs, sans degrader l'experience clavier.

## Etapes a suivre
1. Cartographier les zones cliquables utiles : graphe, liste de fichiers, diff, onglets Branches, barres de navigation, popups.
2. Ajouter un vrai hit-testing par zone dans `src/ui/input.rs` ou un module dedie, en evitant d'enfouir la logique de layout dans le mapper clavier.
3. Definir un comportement clair du clic simple :
   - selectionner un commit ;
   - selectionner un fichier ;
   - changer de focus ;
   - changer d'onglet ou de vue.
4. Ajouter un support minimal du scroll horizontal/vertical dans les panneaux quand la souris est au-dessus de la bonne zone.
5. Rendre le focus visuellement plus explicite dans toutes les vues, surtout Graph et Staging.
6. Revoir les transitions actuelles `Enter`, `Esc`, `Tab` pour qu'elles correspondent au focus visible.
7. S'assurer que les popups modaux bloquent bien les interactions derriere eux.
8. Ajouter des tests au moins sur la logique de mapping des clics et des changements de focus.
9. Mettre a jour l'aide utilisateur avec les interactions souris reellement supportees.

## Validation attendue
- Un clic sur un commit ou un fichier produit un effet utile et previsible.
- Le focus visible correspond toujours a la zone qui recevra la prochaine action clavier.
- Les modales et overlays gardent un comportement coherent.

## Risques / points d'attention
- Il faut eviter de dupliquer les calculs de layout entre le rendu et le hit-testing ; idealement les zones utiles doivent etre centralisees.
