# Clients Git en terminal : que reprendre dans git_sv ?

**Décision :** prendre GitUI comme référence pour apprendre les raccourcis en situation, et lazygit pour retrouver un élément sans quitter sa liste. Le graphe, le staging ligne par ligne, l’historique par chemin et les aperçus d’images existent déjà dans `git_sv` ; les rendre faciles à trouver apporte davantage qu’un panneau supplémentaire.

## Cinq références, cinq usages

Classement par étoiles GitHub, relevées le **21 novembre 2026** via `GET /repos/{owner}/{repo}` (`stargazers_count`). La popularité sert à sélectionner des références, pas à noter leur ergonomie. Les liens pointent vers les versions consultées des documentations. Il s’agit d’une comparaison documentaire avec le code de `git_sv`, pas d’un test de vitesse ni d’une étude utilisateur des cinq clients.

| Dépôt et source | Étoiles | Geste ou choix documenté | Intérêt pour git_sv |
|---|---:|---|---|
| [lazygit](https://github.com/jesseduffield/lazygit/blob/54cc6af1d77b5a6cec3dc9ad0b203afdb4ad3451/README.md#filter) | 83 986 | `/` filtre une vue, notamment les branches ; on peut aussi marquer une référence pour la comparer à une autre. | Retrouver une branche par son nom plutôt que parcourir des dizaines de lignes. Distinguer la sélection du checkout : ici `Entrée` a déjà une autre fonction. |
| [GitUI](https://github.com/gitui-org/gitui/blob/d7214eccb3cac52ce3f8d9ca18f218671821eeee/README.md) | 22 678 | Aide liée au contexte, staging par ligne ou hunk, API Git asynchrone. | L’aide contextuelle répond à un manque réel : dans l’ancien popup, les raccourcis globaux repoussaient les actions du panneau sous le bord inférieur, sans défilement. Ne pas réimplémenter le staging fin. |
| [tig](https://github.com/jonas/tig/blob/d0e095978f15f90f273cb3dc7c496d65d290afe6/README.adoc) | 13 384 | Navigateur Git qui sert aussi de pager pour la sortie de commandes Git. | Une piste distincte de nos sorties `plain`/`json` : lire un flux existant plutôt que rouvrir un dépôt. Moins prioritaire que bien lire un diff dans la TUI. |
| [gitu](https://github.com/altsem/gitu/blob/e82d21ba81cb1ca4d3c325b43cc4b55187294027/README.md) | 2 958 | Client inspiré de Magit : aide à la demande ou persistante ; commit `fixup` et rebase `autosquash`. | Garder un pied de panneau court et une aide complète à la demande. Un `fixup` guidé aurait un intérêt distinct de notre amend du dernier commit. |
| [Serie](https://github.com/lusingander/serie/blob/fb6645f68d294112974da888797aa9d2aca079a1/README.md) | 2 191 | Graphe au centre de la navigation, rendu via les protocoles graphiques du terminal ; aide via `?`. | Prendre le graphe comme point d’entrée, mais garder les caractères du terminal pour l’historique : les protocoles d’image ne doivent pas devenir un prérequis pour lire les commits. |

## Parcours intégrés

- **« Je sais ce que je veux faire, pas quelle touche utiliser. »** `?` ouvre d’abord la rubrique de la vue active. `/` cherche par touche ou libellé dans la langue de l’interface ; les commandes personnalisées sont consultables par nom, pas par contenu shell. Les résultats sont en lecture seule. On peut atteindre la dernière commande même en 80×24.
- **« Je cherche `feature/search`, sans vouloir basculer dessus. »** Dans `3` → Branches, `/` ouvre un sélecteur temporaire. Tapez `FEAT`, parcourez les candidats puis choisissez avec `→`. La liste complète revient avec le détail de la branche choisie, et `HEAD` reste en place. Les distantes n’apparaissent que si `R` les a rendues visibles. Une recherche sans résultat n’a pas de cible de repli.
- **« Je lis un patch, pas la liste des commits. »** Le pied du graphe indique `n/N` pour les hunks et `v` pour la présentation quand le diff a le focus. Le compteur de commits reste visible. En saisie de recherche, le pied présente les commandes de recherche, pas les opérations Git.

- **« Je veux revoir exactement ce qui a changé entre A et B. »** `a` marque A, puis `D` compare A au commit sélectionné dans une vue dédiée. `Esc` revient au graphe et `Ctrl+A` efface la marque. Les deux références restent visibles pendant la lecture ; `C` conserve le résumé par rapport à `HEAD`.

## Prochaines fonctionnalités proposées, non implémentées

| Priorité | Fonctionnalité | Pourquoi / critère de réussite |
|---|---|---|
| 1 | **Préparer un commit `fixup`** | Choisir le commit à corriger, revoir le diff indexé, créer `fixup!` après confirmation. Distinguer la création du commit d’un éventuel autosquash ; ne jamais réécrire automatiquement l’historique publié. |
| 2 | **Mode pager pour les diffs** | Entrée non interactive, par exemple `git diff | git_sv pager` : lire le patch depuis stdin et naviguer entre fichiers et hunks. Ne proposer ni staging ni opération d’écriture pour un patch qui ne provient pas forcément du dépôt courant. |

Avant de lancer le chantier suivant, faire essayer les deux parcours intégrés à des personnes qui ne connaissent pas les touches : retrouvent-elles le raccourci de bisect et le détail d’une branche depuis un terminal 80×24, sans consulter le README ni changer `HEAD` ? C’est un meilleur indicateur que le nombre de fonctionnalités cochées.
