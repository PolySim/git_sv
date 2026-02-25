# STEP-005 : Supprimer les messages de debug qui cassent l'UI

## Problème

Lors d'opérations push, fetch (et potentiellement pull), des messages `[DEBUG]` s'affichent
à l'écran via `eprintln!` et corrompent le rendu TUI. Le terminal en mode raw n'attend pas
de sortie sur stderr, ce qui crée des artefacts visuels.

**Cause racine :** Des `eprintln!("[DEBUG] ...")` sont disséminés dans le code de gestion
SSH et des opérations remote. Il y a aussi des macros `[PERF]` dans utils.

## Fichiers et lignes concernés

### `src/git/remote.rs` — Messages SSH et opérations remote

| Ligne | Message |
|-------|---------|
| ~166 | `[DEBUG] Looking for SSH config for host: {}` |
| ~170 | `[DEBUG] Searching by HostName for: {}` |
| ~174-175 | `[DEBUG] Found match: alias '{}' has HostName '{}'` |
| ~179-180 | `[DEBUG] Using IdentityFile from alias '{}': {:?}` |
| ~197 | `[DEBUG] Found direct config for host: {}` |
| ~199 | `[DEBUG] Using IdentityFile: {:?}` |
| ~212 | `[DEBUG] Falling back to SSH agent` |
| ~218 | `[DEBUG] Falling back to default keys` |
| ~225 | `[DEBUG] Using default key: {:?}` |
| ~241-243 | `[DEBUG] SSH credentials callback - URL: {}, username: {:?}, allowed_types: {:?}` |
| ~246 | `[DEBUG] SSH credentials result: {:?}` |
| ~295-296 | `[DEBUG] Push - URL brute: {}, URL résolue: {}` |
| ~319-320 | `[DEBUG] Push via libgit2 failed: {}, trying CLI fallback` |
| ~523-524 | `[DEBUG] Fetch - URL brute: {}, URL résolue: {}` |
| ~547 | `[DEBUG] Fetch libgit2 échoué: {}, tentative CLI...` |

### `src/utils/mod.rs` — Macros de performance

| Ligne | Message |
|-------|---------|
| ~16, ~29 | `[PERF]` via `eprintln!` |

## Plan de correction

### Option A : Suppression pure (recommandée pour v1)

Supprimer tous les `eprintln!("[DEBUG]...")` dans `src/git/remote.rs`.
Supprimer ou conditionner les `eprintln!("[PERF]...")` dans `src/utils/mod.rs`.

### Option B : Migration vers un logger conditionnel

Remplacer les `eprintln!` par des appels `log::debug!()` / `log::trace!()` avec la crate `log` :

```rust
// AVANT
eprintln!("[DEBUG] Looking for SSH config for host: {}", host);
// APRÈS
log::debug!("Looking for SSH config for host: {}", host);
```

Ces messages ne s'afficheront que si un subscriber est configuré (pas le cas par défaut
en mode TUI). Un fichier de log peut être configuré en option via `env_logger` ou `tracing`.

### Option C : Redirection stderr

Encapsuler stderr pendant les opérations remote pour capturer la sortie, puis la montrer
dans un flash message si nécessaire. Plus complexe et moins propre.

### Vérification

- [ ] Faire un `push` → aucun artefact à l'écran
- [ ] Faire un `fetch` → aucun artefact à l'écran
- [ ] Faire un `pull` → aucun artefact à l'écran
- [ ] Les erreurs légitimes (échec d'auth, remote introuvable) sont toujours affichées en flash message
- [ ] `grep -rn "eprintln" src/` ne retourne aucun résultat (ou uniquement des usages légitimes)
