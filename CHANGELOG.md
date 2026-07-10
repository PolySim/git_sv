# Changelog

## Unreleased

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
