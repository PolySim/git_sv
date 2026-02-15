# Phase 8 — Nouvelles fonctionnalités

## 8.1 Git Push / Pull / Fetch

**Priorité** : Haute — Ce sont des opérations quotidiennes essentielles.

- [ ] **Push** : Pousser la branche courante vers le remote.
  - Keybinding : `P` (majuscule).
  - Afficher un flash message avec le résultat.
  - Gérer le cas où le remote n'est pas configuré.
- [ ] **Pull** : Tirer les changements depuis le remote.
  - Keybinding : `p`.
  - Gérer les conflits éventuels.
- [ ] **Fetch** : Récupérer les refs du remote sans merger.
  - Keybinding : `f`.
  - Mettre à jour la refs map et le graphe.

## 8.2 Recherche / Filtre de commits

**Priorité** : Haute — Indispensable pour naviguer dans un gros historique.

- [ ] **Recherche par message** : `/` pour ouvrir un champ de recherche, filtrer les commits dont le message contient la chaîne.
- [ ] **Recherche par auteur** : Permettre de filtrer par auteur.
- [ ] **Recherche par hash** : Permettre de sauter directement à un commit par son hash.
- [ ] Mettre en surbrillance les résultats de recherche dans le graphe.

## 8.3 Vue Blame / Annotate

**Priorité** : Moyenne — Très utile pour comprendre l'historique d'un fichier.

- [ ] Depuis la liste des fichiers d'un commit, permettre d'ouvrir une vue `blame`.
- [ ] Afficher chaque ligne avec l'auteur et le hash du commit qui l'a introduite.
- [ ] Permettre de naviguer entre les commits depuis le blame.

## 8.4 Cherry-pick

**Priorité** : Moyenne — Opération courante sur les branches.

- [ ] Permettre de cherry-pick le commit sélectionné dans le graphe.
- [ ] Keybinding : `x` ou `C`.
- [ ] Gérer les conflits éventuels.

## 8.5 Commit Amend

**Priorité** : Moyenne — Fréquemment utilisé.

- [ ] Ajouter une option pour amender le dernier commit depuis la vue staging.
- [ ] Pré-remplir le message de commit avec le message existant.

## 8.6 Discard / Checkout de fichier

**Priorité** : Haute — Opération quotidienne.

- [ ] Dans la vue Staging (panneau unstaged), permettre de discard les modifications d'un fichier (`git checkout -- file`).
- [ ] Keybinding : `d` (avec confirmation).
- [ ] Ajouter aussi un "discard all" avec `D`.

## 8.7 Gestion des tags

**Priorité** : Moyenne — Utile pour la gestion de releases.

- [ ] Lister les tags dans la vue Branches (nouvel onglet "Tags").
- [ ] Créer un tag (léger ou annoté) sur le commit sélectionné.
- [ ] Supprimer un tag.
- [ ] Pousser les tags vers le remote.

## 8.8 Rebase interactif simplifié

**Priorité** : Basse — Complexe mais très utile.

- [ ] Permettre de rebase la branche courante sur une autre.
- [ ] Interface simplifiée : pas besoin de tout l'éditeur interactif, mais au moins squash/fixup/reorder.

## 8.9 Diff entre branches

**Priorité** : Moyenne — Utile avant un merge.

- [ ] Depuis la vue Branches, permettre de voir le diff entre deux branches.
- [ ] Afficher le nombre de commits d'écart et les fichiers modifiés.

## 8.10 Configuration / Préférences

**Priorité** : Basse — Nice to have.

- [ ] Fichier de config `~/.config/git_sv/config.toml`.
- [ ] Options configurables :
  - Nombre max de commits à charger.
  - Thème de couleurs.
  - Keybindings personnalisés.
  - Affichage de la date (absolue vs relative).

## 8.11 Intégration avec les forges (GitHub/GitLab)

**Priorité** : Basse — Différenciateur par rapport aux autres outils.

- [ ] Afficher les PRs/MRs associées à une branche.
- [ ] Permettre de créer une PR directement depuis l'outil.
- [ ] Afficher les statuts CI/CD.

## 8.12 Export du graphe

**Priorité** : Basse — Nice to have.

- [ ] Permettre d'exporter le graphe en texte brut (pour le coller dans un document).
- [ ] Éventuellement en SVG ou image.
