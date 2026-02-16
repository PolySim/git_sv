# STEP 01 — Bug : Git Pull échoue avec une URL SSH personnalisée

## Problème

Lorsque l'URL du remote utilise un hostname personnalisé (ex: `github-pro` au lieu de `github.com`) pour différencier les clés SSH entre plusieurs comptes, les opérations `pull` et `fetch` échouent.

## Cause racine

Dans `src/git/remote.rs`, les callbacks d'authentification utilisent `git2::Cred::ssh_key_from_agent()` qui délègue à l'agent SSH. Cependant, `libgit2` ne consulte **pas** le fichier `~/.ssh/config` pour résoudre les alias de hosts. Quand le remote est `git@github-pro:user/repo.git`, la résolution du hostname et la sélection de la bonne clé ne se fait pas correctement.

### Code concerné

```
src/git/remote.rs — lignes 30-33 (push_current_branch)
src/git/remote.rs — lignes 122-125 (fetch_all)
```

Les deux fonctions ont le même callback minimaliste :
```rust
callbacks.credentials(|_url, username_from_url, _allowed_types| {
    git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
});
```

## Plan de correction

### Étape 1 — Créer une fonction utilitaire de résolution SSH

Créer une fonction `resolve_ssh_credentials()` dans `src/git/remote.rs` qui implémente une stratégie multi-niveaux :

1. **Lire `~/.ssh/config`** : Parser le fichier de config SSH pour trouver le `Host` correspondant à l'URL du remote, et en extraire :
   - `IdentityFile` (chemin de la clé privée)
   - `User` (optionnel, par défaut `git`)
2. **Essayer `ssh_key`** : Si un `IdentityFile` est trouvé dans la config, utiliser `git2::Cred::ssh_key()` avec le chemin explicite de la clé.
3. **Fallback `ssh_key_from_agent`** : Si pas de config trouvée, fallback vers l'agent SSH.
4. **Fallback chemin par défaut** : En dernier recours, essayer `~/.ssh/id_rsa`, `~/.ssh/id_ed25519`, etc.

### Étape 2 — Parser le fichier SSH config

Ajouter une fonction de parsing minimal de `~/.ssh/config` :
- Reconnaître les blocs `Host <pattern>`
- Extraire `IdentityFile` et `User`
- Gérer les wildcards basiques (`*`)
- Gérer le `~` dans les chemins (`IdentityFile ~/.ssh/id_github_pro`)

On peut utiliser la crate `ssh2-config` ou parser manuellement (le format est simple).

### Étape 3 — Refactoriser les callbacks

Remplacer les 3 callbacks identiques (push, fetch, pull) par un appel à la fonction utilitaire :

```rust
fn build_remote_callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        resolve_ssh_credentials(url, username_from_url, allowed_types)
    });
    callbacks
}
```

Utiliser `build_remote_callbacks()` dans :
- `push_current_branch()`
- `pull_current_branch()` (via `fetch_all()`)
- `fetch_all()`

### Étape 4 — Améliorer les messages d'erreur

Quand l'authentification échoue, afficher un message clair dans le flash message :
- `"Erreur SSH: clé non trouvée pour 'github-pro'. Vérifiez ~/.ssh/config"`
- Inclure l'URL du remote dans le message pour faciliter le debug.

### Étape 5 — Tests

- Tester le parsing de `~/.ssh/config` avec plusieurs formats (Host simple, IdentityFile avec `~`, etc.)
- Tester le fallback quand aucune config n'est trouvée.
- Tester avec un hostname standard (`github.com`) pour s'assurer qu'il n'y a pas de régression.

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/git/remote.rs` | Ajouter `resolve_ssh_credentials()`, `build_remote_callbacks()`, `parse_ssh_config()` |
| `Cargo.toml` | Éventuellement ajouter une crate de parsing SSH config (optionnel) |

## Priorité

**Haute** — Le pull/fetch ne fonctionne pas du tout pour les utilisateurs multi-comptes SSH.
