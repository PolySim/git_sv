# Phase 6 — Améliorations UI/UX

## 6.1 Dates relatives

**Problème** : Les dates sont affichées en format absolu (`2026-02-15 14:30:00`), ce qui est moins lisible que des dates relatives.

- [ ] Ajouter des dates relatives dans le graphe : "il y a 2h", "hier", "il y a 3 jours".
- [ ] Garder la date absolue dans le panneau de détail.
- [ ] Envisager l'ajout de la crate `timeago` ou implémenter manuellement.

## 6.2 Confirmation pour les actions destructives

**Problème** : Les actions comme `StashDrop`, `BranchDelete`, `WorktreeRemove` s'exécutent directement sans confirmation.

- [ ] Créer un composant `ConfirmDialog` réutilisable.
- [ ] Ajouter un état `PendingConfirmation(action)` dans l'App.
- [ ] Afficher un overlay "Êtes-vous sûr ? (y/n)" avant l'exécution.

## 6.3 Indicateur de chargement

**Problème** : Les opérations longues (construction du graphe sur un gros repo, fetch) bloquent l'UI sans feedback.

- [ ] Ajouter un spinner ou un message "Chargement..." pendant les opérations longues.
- [ ] Envisager l'exécution asynchrone des opérations git lourdes.

## 6.4 Support de la souris

- [ ] Activer `MouseCapture` dans crossterm.
- [ ] Permettre le clic pour sélectionner un commit dans le graphe.
- [ ] Permettre le scroll à la molette dans les panneaux.

## 6.5 Thèmes et couleurs configurables

- [ ] Extraire toutes les couleurs dans une structure `Theme`.
- [ ] Permettre de choisir entre un thème clair et un thème sombre.
- [ ] Éventuellement charger un thème depuis un fichier de configuration.

## 6.6 Légende du graphe

- [ ] Ajouter un petit indicateur de couleur/branche dans un coin du graphe pour montrer quelle couleur correspond à quelle branche.

## 6.7 Panneau de diff amélioré

- [ ] **Side-by-side diff** : Proposer une vue diff côte à côte en plus de la vue unifiée.
- [ ] **Syntax highlighting** : Colorer le code dans le diff selon le langage (via `syntect` ou une crate similaire).
- [ ] **Word diff** : Mettre en surbrillance les mots modifiés au sein d'une ligne, pas seulement la ligne entière.

## 6.8 Barre de navigation entre vues

**Problème** : La navigation entre Graph (1), Staging (2), Branches (3) n'est pas visuellement indiquée.

- [ ] Ajouter un indicateur d'onglet persistant en haut de l'écran montrant la vue active (similaire aux onglets de la vue branches).
