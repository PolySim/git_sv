# STEP 02 — Bug : Impossible de taper certaines lettres dans les champs de saisie

## Problème

Lors de la saisie d'un message de commit (vue Staging, focus `CommitMessage`), il est impossible de taper la lettre `r` (et potentiellement `q` et `?`). Ces caractères sont interceptés par les raccourcis clavier globaux avant d'atteindre le handler de saisie.

## Cause racine

Dans `src/ui/input.rs`, la fonction `map_staging_key()` vérifie les raccourcis globaux **avant** de vérifier le focus actuel :

```rust
fn map_staging_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // ❌ Touches globales vérifiées EN PREMIER
    match key.code {
        KeyCode::Char('q') => return Some(AppAction::Quit),
        KeyCode::Char('r') => return Some(AppAction::Refresh),   // ← Intercepte 'r'
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp), // ← Intercepte '?'
        _ => {}
    }

    // Navigation selon le focus (arrive trop tard pour CommitMessage)
    match state.staging_state.focus {
        StagingFocus::CommitMessage => match key.code {
            KeyCode::Char(c) => Some(AppAction::InsertChar(c)), // ← Jamais atteint pour 'r'
            ...
        },
        ...
    }
}
```

Les lettres `r`, `q`, et `?` sont capturées par le bloc global aux lignes 258-262, ce qui empêche leur insertion dans le message de commit.

**Note** : Le même problème n'existe PAS dans `map_branches_key()` car le mode `Input` y est vérifié en premier (lignes 200-209).

## Plan de correction

### Étape 1 — Vérifier le focus CommitMessage en priorité

Modifier `map_staging_key()` pour vérifier si le focus est sur `CommitMessage` **avant** les raccourcis globaux :

```rust
fn map_staging_key(key: KeyEvent, state: &AppState) -> Option<AppAction> {
    // ✅ Vérifier d'abord si on est en mode saisie de commit
    if state.staging_state.focus == StagingFocus::CommitMessage {
        return match key.code {
            KeyCode::Enter => Some(AppAction::ConfirmCommit),
            KeyCode::Esc => Some(AppAction::CancelCommitMessage),
            KeyCode::Char(c) => Some(AppAction::InsertChar(c)),
            KeyCode::Backspace => Some(AppAction::DeleteChar),
            KeyCode::Left => Some(AppAction::MoveCursorLeft),
            KeyCode::Right => Some(AppAction::MoveCursorRight),
            _ => None,
        };
    }

    // Ensuite, les raccourcis globaux (q, r, ?)
    match key.code {
        KeyCode::Char('q') => return Some(AppAction::Quit),
        KeyCode::Char('r') => return Some(AppAction::Refresh),
        KeyCode::Char('?') => return Some(AppAction::ToggleHelp),
        _ => {}
    }

    // Reste de la navigation...
}
```

### Étape 2 — Audit des autres modes de saisie

Vérifier qu'aucun autre mode de saisie ne souffre du même problème. Points à auditer :

| Mode de saisie | Fonction | Statut actuel |
|---|---|---|
| Staging CommitMessage | `map_staging_key()` | **BUG** — `r`, `q`, `?` interceptés |
| Branches Input | `map_branches_key()` | OK — Input vérifié en premier |
| Search Input | `map_key()` | OK — `search_state.is_active` vérifié en premier |
| Confirmation Dialog | `map_key()` | OK — `pending_confirmation` vérifié en premier |

Seul le `StagingFocus::CommitMessage` est impacté.

### Étape 3 — Tests manuels

Vérifier que les caractères suivants fonctionnent dans le message de commit :
- `r`, `q`, `?` (précédemment bugués)
- Caractères spéciaux : `@`, `#`, `!`, etc.
- Caractères accentués : `é`, `è`, `à`, etc.
- Vérifier que les raccourcis globaux fonctionnent toujours quand le focus **n'est pas** sur CommitMessage.

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/ui/input.rs` | Réordonner les vérifications dans `map_staging_key()` |

## Priorité

**Haute** — Empêche l'écriture de messages de commit contenant des caractères courants.
