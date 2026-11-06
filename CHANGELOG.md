# Changelog

## Unreleased

## 0.5.3 - 2026-11-06

### Changed

- premier écran sans attendre le scan des fichiers ; statut initial collecté en arrière-plan avec indication de chargement
- changement d'onglet sans recharger le graphe ni rescanner le working tree ; vues invalidées après une mutation, un rafraîchissement manuel ou une notification du watcher
- surveillance Git et historique par chemin sur des workers uniques, avec demandes regroupées et résultats bornés
- arborescence construite depuis l'index et le statut déjà disponible, sans double scan ni vérification individuelle de tous les fichiers suivis
- filtres par chemin et listes de fichiers des stashes sans calcul inutile des patches

### Fixed

- gels périodiques de l'interface pendant les scans du working tree sur les gros dépôts
- résultats de fond périmés rejetés après une opération Git ou un changement de dépôt, chemin ou référence
- données invalidées rechargées au retour de l'aide et du blame
- erreurs de lecture du statut visibles pendant les nouvelles tentatives, puis effacées à la récupération

## 0.5.2 - 2026-11-06

### Fixed

- préservation du staging et des changements locaux lors des discard, merges, stashes et annulations
- résolution des conflits sans perte de contexte, de CRLF ni de saut de ligne final ; reprise correcte des rebases
- staging partiel avant le premier commit, chemins littéraux et hunks suivants ; refus des actions partielles sur les diffs tronqués
- ciblage des worktrees, de la branche upstream et du stash confirmé ; index relu après les commandes Git externes
- sélection souris des listes défilées, protection des modales et confirmation destructive limitée au bouton visible
- recherches recalculées après édition et refresh ; recherche CLI dans tout l'historique, y compris HEAD détachée
- rafraîchissement des références existantes, distantes et partagées entre worktrees
- erreurs Git affichées sans fermer la TUI ; restauration du terminal même si son initialisation échoue
- bornes de dates CLI inclusives, validation des arguments et sorties JSON compatibles avec les pipes fermés
- version du paquet alignée avec le tag de publication et vérifiée dans le workflow release

### Changed

- parcours filtré de l'historique en un seul passage ; recherche floue et calcul d'ascendance optimisés
- budgets mémoire en octets pour les diffs, sorties des processus et historiques d'édition
- prévisualisations SVG isolées des fichiers externes ; cache d'image attaché à son contenu réel
- drainage concurrent des sorties Git/GitHub avec délai et nettoyage des processus descendants
- mises à jour git2 0.21, lru 0.18, anyhow, libz-sys et clipboard-win pour leurs correctifs de sûreté
- vérification CI sans prévisualisation d'images et Clippy étendu aux tests

## 0.5.1 - 2026-07-14

### Added

- staging par hunk ou ligne, rebase interactif et annulation via reflog
- tags, comparaison de branches et bisect
- diagnostic des hooks, signatures et sous-modules ; statut des pull requests GitHub
- outils de diff externes, raccourcis et commandes personnalisées

### Changed

- cache de diffs partagé et borné, graphe compact et chargement géométrique de l'historique
- rendu à la demande et réduction des codecs et du poids du binaire

## 0.5.0 - 2026-07-14

### Added

- comparaison, commit par commit, de l'historique d'un fichier ou dossier avec une autre branche depuis la vue Arborescence
- indication `+` / `-` de la branche d'origine pour chaque commit divergent

### Changed

- chargement paresseux de l'historique, des fichiers touches et des diffs dans la vue Arborescence
- suivi des renommages independant sur chaque branche lors d'une comparaison de chemin
- navigation du graphe et de l'arborescence optimisee pour les grands repositories

### Fixed

- latence importante lors de la navigation dans les grands projets
- rechargement incomplet des panneaux Arborescence au clavier, a la souris ou apres rafraichissement
- comparaison devenue obsolete apres un changement de branche

## 0.4.0 - 2026-07-13

### Added

- vue Arborescence pour parcourir les fichiers et dossiers du projet
- recherche rapide tolerante aux fautes sur les chemins
- historique par fichier ou dossier avec fichiers touches et diff detaille
- copie du contenu exact d'un fichier tel qu'il existe dans un commit

### Changed

- dossiers fermes par defaut avec navigation clavier et souris
- fichiers du chemin consulte regroupes en tete des fichiers touches

## 0.3.39 - 2026-07-13

### Added

- theme Solarized natif qui herite du fond et des couleurs ANSI du terminal
- commande `git_sv theme` pour lister et choisir interactivement un theme
- selection directe avec `git_sv theme <dark|light|solarized>` et alias `themes`

### Changed

- sauvegarde de la configuration utilisateur lors d'un changement de theme
- commande de theme utilisable meme en dehors d'un depot Git

## 0.3.38 - 2026-07-10

### Added

- sortie JSON pour `git_sv graph --format json`
- documentation d'architecture et de contribution remise a jour
- kit de communication dans `docs/COMMUNICATION.md`

### Changed

- README reecrit avec un positionnement produit plus clair
- nettoyage de plusieurs modules, helpers et artefacts inutilises
- simplification d'une partie de la couche state / UI / handlers
- refonte des themes clair et sombre avec des contrastes accessibles
- navigation, aide contextuelle, focus et etats vides harmonises dans toute la TUI
- detection des modifications du worktree et des references Git etendue et fiabilisee

### Fixed

- suppression de code mort et d'incoherences documentaires
- reduction des warnings restants sur les zones nettoyees
- crash du selecteur de reset avec certains messages de commit Unicode
- restauration du terminal apres une erreur inattendue dans une build release
- rafraichissement incomplet des diffs apres une modification externe des fichiers
- affichage tronque des raccourcis et controles dans les petits terminaux

## 0.3.37 - 2026-07-09

### Added

- raccourcis d'edition macOS, selection de texte et historique d'annulation
- selecteur rapide permettant de changer de worktree sans quitter l'application

### Changed

- graphe Git enrichi avec des reperes distincts pour HEAD, la selection, les references et la position dans l'historique
- pipeline CI/CD parallelise et mis a jour pour reduire le temps de validation et de publication

### Fixed

- crash lors de la saisie de caracteres accentues dans les recherches et champs de texte
- classification des branches locales contenant un slash dans le graphe Git
