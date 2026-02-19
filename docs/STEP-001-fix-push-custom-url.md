# STEP-001 — Bug : Push ne fonctionne pas avec URL personnalisée

## Problème

Quand le repository utilise une URL remote personnalisée (alias SSH dans `~/.ssh/config`), le push depuis git_sv echoue, alors qu'un simple `git push` en ligne de commande fonctionne car git native se charge de résoudre la config.

## Cause probable

Dans `src/git/remote.rs`, la fonction `push_current_branch()` (ligne 260) crée un **remote temporaire** avec l'URL résolue (`resolve_remote_url`). Cette approche pose problème :

1. **`resolve_remote_url()`** (ligne 239) remplace l'alias SSH par le vrai hostname. Mais les credentials callback (`resolve_ssh_credentials`, ligne 140) cherchent ensuite dans la config SSH avec le hostname résolu — et non l'alias original. La clé SSH associée à l'alias n'est donc plus trouvée correctement.

2. **Le remote temporaire** (`__temp_remote_<pid>`) n'a pas la configuration du remote d'origine (refspecs, push URL, etc.). Git CLI utilise la config complète du remote.

3. **Conflit entre résolution d'URL et résolution de credentials** : L'URL est résolue pour le transport, mais les credentials sont aussi résolues depuis l'URL, ce qui double la résolution et peut la casser.

4. **`push_refspec`** ne supporte pas `--set-upstream` correctement via libgit2 — `remote_push_options` n'est pas équivalent à `git push -u`.

## Fichiers concernés

- `src/git/remote.rs` — `push_current_branch()` (l260), `resolve_remote_url()` (l239), `resolve_ssh_credentials()` (l140)

## Solution proposée

### Option A — Utiliser le remote d'origine sans URL résolue

Ne pas résoudre l'URL et ne pas créer de remote temporaire. Utiliser le remote configuré directement et laisser les callbacks SSH se charger de la résolution (comme le fait git CLI) :

```rust
pub fn push_current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let branch_name = head.shorthand().ok_or_else(/* ... */)?;
    
    let remote_name = /* extraire le remote configuré, fallback "origin" */;
    let mut remote = repo.find_remote(&remote_name)?;
    
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(build_remote_callbacks());
    
    let push_refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[&push_refspec], Some(&mut push_options))?;
    
    Ok(format!("Push de '{}' vers {}", branch_name, remote_name))
}
```

### Option B — Fallback vers `std::process::Command`

Si libgit2 ne supporte pas correctement les alias SSH config, faire un fallback vers `git push` en subprocess :

```rust
use std::process::Command;

pub fn push_current_branch_cli(repo_path: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("push")
        .current_dir(repo_path)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(/* ... */);
    }
    Ok(/* message */)
}
```

### Recommandation

Tenter l'option A d'abord. Si l'alias SSH n'est toujours pas géré correctement par libgit2, implémenter l'option B en fallback. Appliquer la même logique à `fetch_all()` et `pull_current_branch()`.

## Tests

- Tester push sur un repo avec URL standard (`git@github.com:user/repo.git`)
- Tester push sur un repo avec alias SSH (`git@github-perso:user/repo.git` avec `Host github-perso` dans `~/.ssh/config`)
- Tester push sur une nouvelle branche sans upstream
- Tester push avec remote HTTPS
