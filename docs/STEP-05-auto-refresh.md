# STEP-05 : Auto-refresh — Remplacement du refresh manuel par `r`

## Problème

L'utilisateur doit appuyer sur `r` pour rafraîchir les données (commits, status, etc.). L'application devrait détecter automatiquement les changements dans le repository git et rafraîchir sans intervention manuelle.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/handler/mod.rs` | 63-80 | Boucle événementielle — poll input avec timeout |
| `src/handler/mod.rs` | 83-148 | `refresh()` — reconstruction complète du graphe et des données |
| `src/ui/input.rs` | 22-25 | `handle_input_with_timeout` — poll avec timeout configurable |

## Analyse du flux actuel

```
loop {
    terminal.draw(|frame| ui::render(frame, &state));

    let timeout_ms = if flash_message { 100 } else { 250 };
    if let Some(action) = handle_input_with_timeout(&state, timeout_ms)? {
        dispatcher.dispatch(&mut state, action)?;
    }

    state.check_flash_expired();

    if state.dirty {
        self.refresh()?;
    }
}
```

Le timeout de `250ms` est déjà utilisé pour le polling des événements. L'auto-refresh peut être intégré dans cette boucle.

## Solution proposée

### Mécanisme : File watcher sur le répertoire `.git`

1. **Ajouter la dépendance `notify`** dans `Cargo.toml` pour surveiller les changements du filesystem.

2. **Créer un watcher** dans `src/handler/mod.rs` (ou un nouveau module `src/watcher.rs`) :
   - Surveiller le répertoire `.git/` du repository (HEAD, refs, index, etc.)
   - Utiliser un canal (`std::sync::mpsc`) pour signaler les changements au thread principal
   - Appliquer un debounce de ~500ms pour éviter les rafraîchissements excessifs

3. **Modifier la boucle événementielle** dans `EventHandler::run()` :
   ```rust
   pub fn run(&mut self, terminal: &mut Terminal<...>) -> Result<()> {
       let (tx, rx) = std::sync::mpsc::channel();
       let watcher = start_git_watcher(&self.state.repo_path, tx)?;

       loop {
           terminal.draw(...);

           // Vérifier les changements git (non bloquant)
           if rx.try_recv().is_ok() {
               self.state.dirty = true;
           }

           if let Some(action) = handle_input_with_timeout(&self.state, timeout_ms)? {
               self.dispatcher.dispatch(&mut self.state, action)?;
           }

           if self.state.dirty {
               self.refresh()?;
           }
       }
   }
   ```

4. **Fichiers à surveiller** dans `.git/` :
   - `HEAD` — changement de branche
   - `refs/` — nouveaux commits, branches, tags
   - `index` — fichiers stagés/unstagés
   - `MERGE_HEAD`, `REBASE_HEAD` — opérations en cours

5. **Conserver le `r` manuel** comme fallback en cas de problème avec le watcher.

### Alternative sans dépendance externe : polling périodique

Si on ne veut pas ajouter `notify`, on peut utiliser un polling basé sur le timestamp de modification de `.git/HEAD` et `.git/index` :

```rust
// Dans la boucle, toutes les 2 secondes
if last_check.elapsed() > Duration::from_secs(2) {
    let head_mtime = std::fs::metadata(".git/HEAD")?.modified()?;
    let index_mtime = std::fs::metadata(".git/index")?.modified()?;
    if head_mtime != cached_head_mtime || index_mtime != cached_index_mtime {
        self.state.dirty = true;
        cached_head_mtime = head_mtime;
        cached_index_mtime = index_mtime;
    }
    last_check = Instant::now();
}
```

## Ordre d'implémentation

1. Ajouter la dépendance `notify` (ou implémenter le polling simple)
2. Créer le module `src/watcher.rs` avec la logique de surveillance
3. Intégrer dans la boucle événementielle de `src/handler/mod.rs`
4. Ajouter un debounce pour éviter les refreshs excessifs
5. Conserver la touche `r` comme refresh manuel
6. Tester : faire un commit dans un autre terminal → l'app se met à jour automatiquement

## Critère de validation

- L'application détecte automatiquement un nouveau commit et met à jour le graphe
- L'application détecte les changements de branche
- L'application détecte les modifications du working directory (stage/unstage)
- Le refresh automatique n'impacte pas les performances (debounce)
- La touche `r` fonctionne toujours comme fallback
