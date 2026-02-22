# STEP-01 : Corriger la résolution du nom de remote (fetch/push/pull)

## Problème

L'erreur `remote 'refs' does not exist; class=Config (7); code=NotFound (-3)` apparaît lors de fetch, push ou pull.

**Cause racine** : `repo.branch_upstream_name()` retourne le chemin complet de la référence (ex: `"refs/remotes/origin/main"`). Le code fait `.split('/').next()` ce qui donne `"refs"` au lieu de `"origin"`.

## Fichiers à modifier

- `src/git/remote.rs`

## Corrections

### 1. Fonction `push_current_branch` (~lignes 273-285)

Le code actuel :
```rust
let remote_name = repo
    .branch_upstream_name(&format!("refs/heads/{}", branch_name))
    .ok()
    .and_then(|name| name.as_str().map(|s| s.to_string()))
    .unwrap_or_else(|| "origin".to_string());

let remote_name = remote_name
    .split('/')
    .next()                    // ← BUG : retourne "refs"
    .unwrap_or("origin")
    .to_string();
```

Doit devenir :
```rust
let remote_name = repo
    .branch_upstream_name(&format!("refs/heads/{}", branch_name))
    .ok()
    .and_then(|name| name.as_str().map(|s| s.to_string()))
    .and_then(|name| {
        name.strip_prefix("refs/remotes/")
            .and_then(|rest| rest.split('/').next())
            .map(|s| s.to_string())
    })
    .unwrap_or_else(|| "origin".to_string());
```

### 2. Fonction `fetch_all` (~lignes 470-484)

Même correction : extraire le segment après `refs/remotes/` au lieu du premier segment.

### 3. Fonction `get_default_remote` (~lignes 515-535)

Même correction.

### 4. Refspec hardcodé dans `fetch_all` (~ligne 500)

Le refspec est hardcodé sur `origin` :
```rust
remote.fetch(
    &[&format!("refs/heads/*:refs/remotes/origin/*")],  // ← BUG
    ...
)?;
```

Utiliser le nom du remote résolu, ou mieux, passer un slice vide `&[]` pour utiliser les refspecs configurés par défaut du remote :
```rust
remote.fetch(&[] as &[&str], Some(&mut fetch_options), None)?;
```

### 5. Factoriser l'extraction du remote

Créer une fonction utilitaire pour éviter la duplication :
```rust
fn resolve_remote_name(repo: &Repository, branch_name: &str) -> String {
    repo.branch_upstream_name(&format!("refs/heads/{}", branch_name))
        .ok()
        .and_then(|name| name.as_str().map(|s| s.to_string()))
        .and_then(|name| {
            name.strip_prefix("refs/remotes/")
                .and_then(|rest| rest.split('/').next())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "origin".to_string())
}
```

## Vérification

```bash
cargo test
# Tester manuellement : fetch, push, pull sur un repo avec upstream configuré
```
