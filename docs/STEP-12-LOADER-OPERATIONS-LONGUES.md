# STEP 12 — Loader pour les opérations longues

**Priorité** : Haute
**Effort estimé** : Élevé (introduction du threading)
**Impact** : Expérience utilisateur critique — l'UI gèle pendant les opérations réseau

---

## Constat

Toutes les opérations git (push, pull, fetch, merge, commit, cherry-pick, blame) s'exécutent **de manière synchrone sur le thread principal**. Pendant une opération réseau (push, pull, fetch), l'UI est **complètement gelée** : pas de rendu, pas d'input, pas de feedback visuel. L'utilisateur ne sait pas si l'opération est en cours ou si l'application a planté.

**Infrastructure existante mais inutilisée** :
- `src/ui/loading.rs` : Un widget `LoadingSpinner` complet avec animation braille (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) — **100% dead code** (`#![allow(dead_code)]`)
- `AppState.loading_spinner: Option<LoadingSpinner>` (ligne 132) — **toujours `None`**
- `render_overlay()` et `render_inline()` — **jamais appelés**

**L'architecture actuelle ne permet pas d'afficher un spinner** car la boucle d'événements est bloquée :
```
loop {
    terminal.draw(...)            // 1. Render
    watcher.check_changed()?      // 2. Watch
    handle_input_with_timeout()   // 3. Input (100-250ms timeout)
    dispatcher.dispatch(action)   // 4. *** BLOQUE ICI pour push/pull/fetch ***
    state.check_flash_expired()   // 5. Flash
    if state.dirty { refresh() }  // 6. Refresh
}
```

Pour afficher un spinner animé pendant une opération, il faut que la boucle de rendu continue de tourner pendant que l'opération s'exécute en arrière-plan.

### Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/ui/loading.rs` | Widget `LoadingSpinner` (dead code) |
| `src/handler/mod.rs` | Boucle d'événements `EventHandler::run()` (ligne 52) |
| `src/handler/git.rs` | Handlers push/pull/fetch (lignes 37, 55, 96) |
| `src/handler/dispatcher.rs` | `dispatch()` — exécution synchrone des actions |
| `src/git/remote.rs` | `push_current_branch()`, `pull_current_branch_with_result()`, `fetch_all()` |
| `src/state/mod.rs` | `AppState.loading_spinner` (ligne 132) |
| `src/app.rs` | Point d'entrée de l'application |

---

## Actions à mener

### 12.1 — Introduire un canal pour les opérations en arrière-plan (priorité haute)

Utiliser `std::sync::mpsc` pour communiquer entre le thread d'opération et le thread principal :

**Nouveau fichier `src/handler/background.rs`** :
```rust
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Résultat d'une opération en arrière-plan.
#[derive(Debug)]
pub enum BackgroundResult {
    PushComplete(Result<String, String>),
    PullComplete(Result<PullResult, String>),
    FetchComplete(Result<String, String>),
    // Extensible pour d'autres opérations
}

/// Wrapper pour les données nécessaires au thread (non-Send workaround).
/// Comme git2::Repository n'est pas Send, utiliser le chemin du repo
/// et ouvrir une nouvelle instance dans le thread.
pub struct BackgroundRunner {
    pub sender: Sender<BackgroundResult>,
    pub receiver: Receiver<BackgroundResult>,
}

impl BackgroundRunner {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    /// Lance un push en arrière-plan.
    pub fn spawn_push(&self, repo_path: PathBuf) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            let repo = Repository::open(&repo_path).unwrap();
            let result = push_current_branch(&repo);
            let _ = tx.send(BackgroundResult::PushComplete(
                result.map(|_| "Push réussi".to_string())
                      .map_err(|e| e.to_string())
            ));
        });
    }

    // spawn_pull(), spawn_fetch() similaires...
}
```

**Note importante** : `git2::Repository` n'implémente pas `Send`. Il faut :
- Soit ouvrir un nouveau `Repository` dans le thread avec le chemin
- Soit utiliser les fallbacks CLI (`push_current_branch_cli()`, `fetch_all_cli()`) qui n'ont pas cette limitation (ils utilisent `std::process::Command`)

→ **Recommandation** : Utiliser les versions CLI pour les opérations en arrière-plan. Elles sont déjà implémentées comme fallback dans `src/git/remote.rs`.

### 12.2 — Modifier la boucle d'événements (priorité haute)

**`src/handler/mod.rs`** — Modifier `EventHandler::run()` pour vérifier les résultats d'arrière-plan :

```rust
pub fn run(&mut self) -> Result<()> {
    loop {
        // 1. Render (inclut le spinner si loading)
        terminal.draw(|frame| {
            ui::render(frame, &self.state);
            // Si un spinner est actif, le rendre en overlay
            if let Some(spinner) = &self.state.loading_spinner {
                spinner.render_overlay(frame, frame.area());
            }
        })?;

        // 2. Vérifier les résultats d'opérations en arrière-plan
        if let Ok(result) = self.background.receiver.try_recv() {
            self.handle_background_result(result)?;
        }

        // 3. Tick du spinner (mettre à jour l'animation)
        if let Some(spinner) = &mut self.state.loading_spinner {
            spinner.tick();
        }

        // 4. Input (timeout court pour l'animation du spinner)
        let timeout = if self.state.loading_spinner.is_some() { 80 } else { 250 };
        let action = handle_input_with_timeout(&self.state, timeout)?;

        // 5. Pendant le loading, ignorer les actions sauf Quit
        if self.state.loading_spinner.is_some() {
            match action {
                Some(AppAction::Quit) => { /* permettre de quitter */ }
                _ => continue, // Ignorer les inputs pendant le chargement
            }
        }

        // 6. Dispatch normal
        if let Some(action) = action {
            self.dispatcher.dispatch(&mut self.state, action)?;
        }

        // ... reste identique
    }
}
```

### 12.3 — Modifier les handlers pour lancer en arrière-plan (priorité haute)

**`src/handler/git.rs`** — Transformer `handle_push()` :

```rust
fn handle_push(state: &mut AppState, background: &BackgroundRunner) -> Result<()> {
    // Activer le spinner
    state.loading_spinner = Some(LoadingSpinner::new("Push en cours...".to_string()));
    // Lancer en arrière-plan
    background.spawn_push(state.repo.path().to_path_buf());
    Ok(())
}
```

**`src/handler/mod.rs`** — Traiter les résultats :
```rust
fn handle_background_result(&mut self, result: BackgroundResult) -> Result<()> {
    // Désactiver le spinner
    self.state.loading_spinner = None;

    match result {
        BackgroundResult::PushComplete(Ok(msg)) => {
            self.state.set_flash_message(msg);
            self.state.mark_dirty();
        }
        BackgroundResult::PushComplete(Err(err)) => {
            self.state.set_flash_message(format!("Erreur push : {}", err));
        }
        // ... autres cas
    }
    Ok(())
}
```

### 12.4 — Activer le widget LoadingSpinner (priorité haute)

Le widget dans `src/ui/loading.rs` est prêt. Il faut :

- Retirer `#![allow(dead_code)]`
- Appeler `render_overlay()` dans la boucle de rendu principale quand `state.loading_spinner.is_some()`
- Vérifier que l'animation fonctionne (le `tick()` incrémente `frame_index` pour faire tourner les caractères braille)
- Réduire le timeout de `handle_input_with_timeout()` à ~80ms quand le spinner est actif pour une animation fluide

### 12.5 — Opérations à rendre asynchrones (priorité par importance)

| Opération | Priorité | Durée typique | Fichier |
|-----------|----------|---------------|---------|
| `push` | Haute | 1-10s | `git/remote.rs:233` |
| `pull` | Haute | 1-10s | `git/remote.rs:289` |
| `fetch` | Haute | 1-10s | `git/remote.rs:308` |
| `merge` | Moyenne | < 1s | `git/merge.rs:23` |
| `blame` | Moyenne | 0.5-5s | `git/blame.rs` |
| `refresh` (build_graph) | Basse | < 0.5s | `handler/mod.rs:77` |
| `commit` | Basse | < 0.1s | `git/commit.rs:55` |

→ Se concentrer d'abord sur push, pull, fetch qui sont les plus lents.

### 12.6 — Gérer l'annulation (priorité basse)

Permettre d'annuler une opération en cours avec `Esc` :

- Ajouter un `AtomicBool` partagé entre le thread principal et le thread d'opération
- Les callbacks de progression de `git2` (`RemoteCallbacks::transfer_progress`, etc.) vérifient ce flag
- Pour les versions CLI, utiliser `Child::kill()` pour tuer le processus

### 12.7 — Indicateur de progression (priorité basse)

Pour les opérations git2 qui supportent les callbacks de progression :

```rust
let mut callbacks = RemoteCallbacks::new();
callbacks.transfer_progress(|stats| {
    let progress = format!(
        "Objets: {}/{} | Deltas: {}/{}",
        stats.received_objects(), stats.total_objects(),
        stats.indexed_deltas(), stats.total_deltas()
    );
    // Envoyer au thread principal via un canal
    let _ = tx.send(BackgroundProgress::Update(progress));
    true // continuer
});
```

---

## Architecture proposée

```
┌──────────────────────────────────────────────┐
│              Thread Principal                 │
│                                              │
│  ┌──────────┐   ┌───────────┐   ┌────────┐  │
│  │ Rendu UI │──▶│ Event Loop│──▶│Dispatch│  │
│  │(+spinner)│   │try_recv() │   │        │  │
│  └──────────┘   └─────┬─────┘   └───┬────┘  │
│                       │              │       │
│                       │     spawn_push()     │
│                       │              │       │
│                       ▼              ▼       │
│              ┌─────────────┐  ┌──────────┐   │
│              │ Résultat OK │  │ Channel  │   │
│              │ ou Erreur   │  │  (mpsc)  │   │
│              └──────┬──────┘  └────┬─────┘   │
│                     │              │         │
└─────────────────────┼──────────────┼─────────┘
                      │              │
                      ▼              ▼
              ┌──────────────────────────┐
              │    Thread Background     │
              │                          │
              │  Repository::open(path)  │
              │  push / pull / fetch     │
              │  tx.send(result)         │
              └──────────────────────────┘
```

---

## Critères de validation

- [ ] Push affiche un spinner pendant l'exécution
- [ ] Pull affiche un spinner pendant l'exécution
- [ ] Fetch affiche un spinner pendant l'exécution
- [ ] L'UI reste réactive (les frames se redessinent) pendant l'opération
- [ ] Le résultat (succès/erreur) s'affiche en flash message après l'opération
- [ ] Les inputs sont bloqués pendant l'opération (sauf Quit)
- [ ] Pas de race condition ni de crash (tester avec des opérations réseau lentes)
- [ ] `cargo clippy` propre
- [ ] `cargo test` passe
