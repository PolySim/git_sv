# STEP 07 — Améliorations fonctionnelles et UX

**Priorité** : Basse-Moyenne (à traiter après les STEP 01-06)
**Effort estimé** : Variable par feature
**Impact** : Expérience utilisateur, fonctionnalités manquantes

---

## Constat

Le projet est fonctionnel et riche en features (graphe, staging, branches, conflits, blame, stash, worktrees, search, filter). Il reste quelques améliorations pour atteindre un niveau de polish professionnel.

---

## Actions à mener

### 7.1 — Internationalisation (i18n)

Actuellement, toutes les chaînes de l'UI sont en français en dur dans le code. Pour toucher un public plus large :

- Extraire les chaînes dans un système de traduction (ex: `fluent-rs` ou simple fichier de constantes)
- Supporter au minimum français et anglais
- Détecter la locale système

**Note** : Si le projet reste ciblé français uniquement, cette étape peut être ignorée.

### 7.2 — Configuration utilisateur

Le projet n'a aucun fichier de configuration utilisateur. Ajouter un `~/.config/git_sv/config.toml` pour :

- Thème (forcer dark/light au lieu de l'auto-détection)
- Nombre max de commits à charger (actuellement hardcodé à 200)
- Keybindings custom
- Langue de l'interface

### 7.3 — Améliorer le file watcher

Le watcher actuel est un polling toutes les 2 secondes sur les mtimes de fichiers. Considérer :

- Utiliser la crate `notify` pour du vrai filesystem watching (inotify/FSEvents/ReadDirectoryChangesW)
- Réduire la latence de détection des changements
- Supporter les événements granulaires (quels fichiers ont changé)

### 7.4 — Opérations asynchrones

Les opérations réseau (push/pull/fetch) sont synchrones et bloquent l'UI. Considérer :

- Exécuter les opérations longues dans un thread séparé
- Afficher un indicateur de progression (le composant `loading.rs` existe déjà)
- Permettre l'annulation des opérations en cours

### 7.5 — Améliorer la vue Blame

La vue blame actuelle (`src/ui/blame_view.rs`, 178 lignes) est basique. Améliorations possibles :

- Navigation dans l'historique du fichier (voir les blames des commits précédents)
- Copie du hash de commit d'une ligne
- Lien vers le commit dans le graphe

### 7.6 — Supporter plus de protocoles remote

`src/git/remote.rs` supporte SSH et credentials. Vérifier/ajouter :

- Support HTTPS avec token
- Support du credential helper de git (`git credential-*`)
- Support des clés SSH avec passphrase (callback)

### 7.7 — Pagination du graphe

`MAX_COMMITS = 200` limite le nombre de commits chargés. Pour les grands repositories :

- Implémenter un lazy loading au scroll (charger plus de commits quand on approche de la fin)
- Afficher un indicateur "X commits chargés sur Y total"

### 7.8 — Améliorer le mode non-interactif (`log`)

La commande `git_sv log` est très basique. Enrichir :

- Support de `--format` pour le formatage de sortie
- Support de `--author`, `--since`, `--until` pour le filtrage
- Sortie colorée par défaut avec `--no-color` option
- Support du pipe (détecter si stdout est un TTY)

### 7.9 — Créer un crate library (`lib.rs`)

Actuellement le projet est un binary-only crate (`main.rs`). Extraire la logique dans un `lib.rs` permettrait :

- De réutiliser les modules git dans d'autres outils
- De faciliter les tests d'intégration
- De publier sur crates.io en tant que bibliothèque

---

## Critères de validation

Chaque amélioration est indépendante. Valider individuellement :

- [ ] Feature implémentée
- [ ] Tests ajoutés
- [ ] Documentation mise à jour
- [ ] `cargo clippy` propre
