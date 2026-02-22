# STEP-06 — Corriger le fetch/push avec les alias SSH

## Problème

Lors d'un fetch/push via une URL utilisant un alias SSH (géré par `~/.ssh/config`), l'erreur suivante apparaît :

```
Erreur lors du fetch: Erreur git : failed to resolve address for github-pro:
nodename nor servname provided, or not known; class=Net (12)
[DEBUG] Fetch - Using remote 'origin' with URL: git@github-pro:Faire-Savoir-Tourinsoft/monorepo-v1.git
```

### Cause racine

1. **`resolve_remote_url()` est du code mort.** La fonction existe dans `git/remote.rs` et fait exactement ce qu'il faut (lire `~/.ssh/config`, remplacer l'alias `github-pro` par le vrai hostname `github.com`), mais **elle n'est appelée nulle part** dans le codebase.

2. **libgit2 ne résout pas les alias SSH.** Contrairement à la commande `ssh`/`git` native qui lit `~/.ssh/config` pour résoudre `Host github-pro → HostName github.com`, libgit2 utilise sa propre stack réseau et essaie de résoudre `github-pro` en DNS directement → échec.

3. **Le callback SSH (`resolve_ssh_credentials`) arrive trop tard.** Il parse bien `~/.ssh/config` pour trouver la bonne clé SSH, mais le callback de credentials n'est invoqué qu'après la connexion TCP. Comme la résolution DNS échoue avant la connexion, le callback n'est jamais atteint.

4. **Pas de fallback CLI pour le fetch.** `push_current_branch()` a un fallback CLI (`push_current_branch_cli()` qui exécute `git push`), mais `fetch_all()` n'a **aucun fallback** et propage directement l'erreur libgit2.

## Fichiers concernés

| Fichier | Modification |
|---------|-------------|
| `src/git/remote.rs` | Appeler `resolve_remote_url()` dans `fetch_all()` et `push_current_branch()`, ajouter un fallback CLI pour le fetch |
| `src/handler/git.rs` | Éventuellement adapter la gestion d'erreur pour le fallback |

## Corrections à apporter

### Option A : Réécrire l'URL avant le fetch/push (recommandé en complément de B)

Dans `fetch_all()`, utiliser `resolve_remote_url()` pour réécrire l'URL :

```rust
pub fn fetch_all(repo: &Repository) -> Result<()> {
    let remote_name = resolve_remote_name(repo)?;
    let remote = repo.find_remote(&remote_name)?;
    let raw_url = remote.url().unwrap_or("").to_string();
    let resolved_url = resolve_remote_url(&raw_url);

    eprintln!("[DEBUG] Fetch - URL brute: {}, URL résolue: {}", raw_url, resolved_url);

    let mut fetch_remote = if resolved_url != raw_url {
        // L'URL a été réécrite, utiliser un remote anonyme
        repo.remote_anonymous(&resolved_url)?
    } else {
        repo.find_remote(&remote_name)?
    };

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(build_remote_callbacks());
    fetch_remote.fetch(&[] as &[&str], Some(&mut fetch_options), None)?;
    Ok(())
}
```

Appliquer la même logique dans `push_current_branch()`.

### Option B : Ajouter un fallback CLI pour le fetch (recommandé)

Comme pour `push_current_branch_cli()`, ajouter un `fetch_all_cli()` :

```rust
fn fetch_all_cli() -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["fetch", "--all"])
        .output()
        .map_err(|e| GitSvError::Git(git2::Error::from_str(&e.to_string())))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitSvError::Git(git2::Error::from_str(&stderr)));
    }
    Ok(())
}
```

Puis dans `fetch_all()`, ajouter le fallback :

```rust
pub fn fetch_all(repo: &Repository) -> Result<()> {
    // ... tentative libgit2 ...
    match fetch_remote.fetch(...) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("[DEBUG] Fetch libgit2 échoué: {}, tentative CLI...", e);
            fetch_all_cli()
        }
    }
}
```

### Option C : Les deux combinées (recommandé)

Combiner A et B pour une robustesse maximale :
1. D'abord réécrire l'URL (résout le cas courant des alias SSH)
2. Si ça échoue quand même, fallback sur `git fetch` CLI (gère tous les cas SSH avancés : ProxyCommand, ProxyJump, Port, etc.)

### Appliquer la même logique au pull

`pull_current_branch_with_result()` appelle `fetch_all()` en interne, donc il bénéficiera automatiquement du fix. Vérifier que c'est bien le cas.

## Vérification

- `cargo build` compile
- `cargo clippy` sans warning
- Tester avec un remote utilisant un alias SSH (`git@github-pro:...`) :
  - `fetch` fonctionne sans erreur
  - `push` fonctionne sans erreur
  - `pull` fonctionne sans erreur
- Tester avec un remote utilisant une URL directe (`git@github.com:...`) : toujours fonctionnel
- Tester avec un remote HTTPS : toujours fonctionnel
- Vérifier que les credentials SSH sont correctement résolues (bonne clé utilisée)
