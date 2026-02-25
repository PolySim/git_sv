# STEP 04 — Robustesse et gestion des erreurs

**Priorité** : Moyenne-Haute
**Effort estimé** : 2 jours
**Impact** : Stabilité, expérience utilisateur

---

## Constat

Le projet utilise `thiserror` pour les erreurs custom et `anyhow` au top-level, ce qui est une bonne base. Cependant, certains patterns fragilisent la robustesse.

---

## Actions à mener

### 4.1 — Auditer les `unwrap()` en code de production

Rechercher tous les appels `unwrap()` en dehors des modules de test. Les cas identifiés :

- `src/state/cache.rs` — `DiffCache::new()` utilise un `unwrap()` imbriqué
- `src/ui/theme.rs` — Détection du thème avec `unwrap()`
- Autres endroits potentiels dans les modules `git/`

Pour chaque occurrence :
- Remplacer par `?` si dans une fonction qui retourne `Result`
- Remplacer par `unwrap_or_default()` ou `unwrap_or_else()` si un fallback est acceptable
- Documenter avec un commentaire si le `unwrap()` est réellement safe (invariant prouvé)

### 4.2 — Auditer les `expect()` en code de production

Même démarche pour `expect()`. Les `expect()` sont acceptables uniquement si :
- L'invariant est documenté
- Le message d'erreur est informatif

### 4.3 — Améliorer les messages d'erreur utilisateur

Le fichier `src/error_display.rs` fournit des fonctions de formatage (`format_error_message`, etc.) mais certaines erreurs git2 sont renvoyées brutes à l'utilisateur.

- Wrapper les erreurs git2 les plus courantes avec des messages contextuels en français
- Exemple : "Impossible de push : pas de remote configuré" au lieu d'un message libgit2 cryptique

### 4.4 — Gestion des opérations réseau

Les opérations `push`, `pull`, `fetch` dans `src/git/remote.rs` (585 lignes) peuvent échouer pour des raisons réseau. S'assurer que :

- Les timeouts sont configurés
- Les erreurs d'authentification SSH sont clairement reportées
- L'UI ne se bloque pas pendant une opération réseau (considérer un feedback de loading)

### 4.5 — Protéger contre les repositories corrompus

Tester le comportement de l'application quand :
- `.git/HEAD` est corrompu
- L'index git est verrouillé (`.git/index.lock` présent)
- Le repository a des submodules non initialisés
- Le répertoire courant n'est pas un repo git

Ajouter des tests pour ces cas limites.

### 4.6 — Vérifier la gestion du file watcher

Le watcher (`src/watcher.rs`) utilise du polling toutes les 2 secondes. Points à vérifier :

- Que se passe-t-il si `.git/` est supprimé pendant l'exécution ?
- Le debounce de 500ms est-il suffisant pour les opérations longues (rebase) ?
- Envisager `notify` crate pour du vrai filesystem watching (inotify/FSEvents) au lieu du polling

---

## Critères de validation

- [ ] Zéro `unwrap()` injustifié en code de production
- [ ] Tous les `expect()` ont un message clair et un invariant documenté
- [ ] Les erreurs réseau sont gérées avec feedback utilisateur
- [ ] Tests ajoutés pour les cas limites (repo corrompu, pas de remote, etc.)
- [ ] Le file watcher gère gracieusement la suppression de `.git/`
