# STEP-03 : Navigation entre occurrences en recherche basique

## Problème

Quand la recherche est fermée (après avoir tapé `/query` puis `Esc` ou `Enter`), les touches `n` et `N` pour naviguer entre les résultats de recherche **ne fonctionnent pas**.

## Fichiers concernés

| Fichier | Lignes | Rôle |
|---------|--------|------|
| `src/ui/input.rs` | 79-90 | Keybindings quand `search_state.is_active` — intercepte TOUTES les touches |
| `src/ui/input.rs` | 198-199 | `n` → `NextSearchResult`, `N` → `PrevSearchResult` (en mode normal) |
| `src/handler/search.rs` | 49-52 | `handle_close` — **efface `results` et `query`** |
| `src/handler/search.rs` | 72-87 | `handle_next_result` / `handle_previous_result` — fonctionnent correctement |

## Analyse

Le problème est dans `handle_close` (`src/handler/search.rs:49-52`) :

```rust
fn handle_close(state: &mut AppState) -> Result<()> {
    state.search_state.is_active = false;
    state.search_state.query.clear();     // ← Efface la requête
    state.search_state.results.clear();   // ← Efface les résultats !
    Ok(())
}
```

Quand l'utilisateur ferme la recherche avec `Esc` ou `Enter`, les résultats sont perdus. Les touches `n`/`N` vérifient `!state.search_state.results.is_empty()` qui est toujours faux après fermeture.

Les keybindings `n`/`N` sont bien définis en mode normal (`input.rs:198-199`) et routés vers `NextSearchResult`/`PrevSearchResult`, mais le handler ne trouve plus de résultats.

## Solution proposée

1. **Modifier `src/handler/search.rs`** — `handle_close` : ne PAS effacer les résultats à la fermeture.

   ```rust
   fn handle_close(state: &mut AppState) -> Result<()> {
       state.search_state.is_active = false;
       // NE PAS effacer query et results pour permettre n/N
       Ok(())
   }
   ```

2. **Modifier `src/handler/search.rs`** — `handle_open` : effacer les anciens résultats uniquement à l'ouverture d'une nouvelle recherche (déjà fait dans `handle_open`).

3. **Optionnel** : ajouter un indicateur visuel dans la status bar ou help bar montrant que des résultats de recherche sont actifs (ex: `[Recherche: 3/15]`).

## Ordre d'implémentation

1. Modifier `handle_close` pour conserver `query` et `results`
2. Vérifier que `handle_open` réinitialise bien tout (déjà le cas)
3. Tester : `/query` → `Enter` → `n` devrait naviguer au résultat suivant
4. Optionnel : afficher un indicateur de résultats actifs dans la help bar

## Critère de validation

- Après avoir fermé la recherche, `n` navigue au résultat suivant
- Après avoir fermé la recherche, `N` navigue au résultat précédent
- Le compteur `X/Y` se met à jour correctement pendant la navigation
- Ouvrir une nouvelle recherche (`/`) efface les anciens résultats
