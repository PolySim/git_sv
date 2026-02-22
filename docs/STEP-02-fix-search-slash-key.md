# STEP-02 : Corriger la recherche / filtre (touche '/')

## Problème

Quand l'utilisateur appuie sur `/`, l'app entre en mode recherche mais :
1. Aucun caractère tapé n'est enregistré dans `search_state.query`
2. Aucune barre de recherche n'est affichée dans l'UI
3. Toutes les touches sont interceptées et avalées silencieusement (sauf Échap)

**Cause racine** : Le mode recherche est incomplet. Les caractères tapés produisent `AppAction::InsertChar(c)` qui est routé vers `EditHandler`, lequel ne modifie que `staging_state.commit_message`. Personne n'écrit dans `search_state.query`. De plus, aucun rendu UI n'est prévu pour la barre de recherche.

## Fichiers à modifier

- `src/ui/input.rs` — Mapper les touches en mode recherche vers des actions dédiées
- `src/state/action/search.rs` — Ajouter des variants `InsertChar(char)` et `DeleteChar` à `SearchAction`
- `src/state/action/mod.rs` — Ajouter des variants legacy si nécessaire (ou utiliser le wrapping `Search(...)`)
- `src/handler/search.rs` — Implémenter l'insertion/suppression de caractères dans `search_state.query`
- `src/handler/dispatcher.rs` — Router les nouvelles actions
- `src/ui/mod.rs` — Rendre la barre de recherche quand `search_state.is_active`
- Possiblement créer `src/ui/search_bar.rs` — Widget de barre de recherche

## Corrections

### 1. Ajouter les variants à `SearchAction` (`src/state/action/search.rs`)

```rust
pub enum SearchAction {
    Open,
    Close,
    InsertChar(char),   // NOUVEAU
    DeleteChar,         // NOUVEAU
    NextResult,
    PreviousResult,
    ChangeType,
    Execute,
}
```

### 2. Modifier le mapping clavier (`src/ui/input.rs`, ~lignes 59-73)

Quand `search_state.is_active`, mapper :
- `KeyCode::Char(c)` → `AppAction::Search(SearchAction::InsertChar(c))`
- `KeyCode::Backspace` → `AppAction::Search(SearchAction::DeleteChar)`
- `KeyCode::Enter` → `AppAction::Search(SearchAction::Execute)` (puis navigation dans les résultats)
- `KeyCode::Esc` → `AppAction::Search(SearchAction::Close)`
- `KeyCode::Tab` → `AppAction::Search(SearchAction::ChangeType)`

### 3. Implémenter dans `SearchHandler` (`src/handler/search.rs`)

```rust
SearchAction::InsertChar(c) => {
    state.search_state.query.push(c);
    state.search_state.cursor += 1;
    // Optionnel : exécuter la recherche incrémentale ici
}
SearchAction::DeleteChar => {
    if state.search_state.cursor > 0 {
        state.search_state.cursor -= 1;
        state.search_state.query.remove(state.search_state.cursor);
    }
}
```

### 4. Rendre la barre de recherche (`src/ui/mod.rs`)

Dans `render_graph_view()`, si `state.search_state.is_active`, afficher une barre de recherche en bas de l'écran (au-dessus ou à la place de la help_bar) montrant :
- Le préfixe `/` ou `search:`
- Le texte de la query avec curseur
- Le type de recherche (Message/Author/Hash)
- Le nombre de résultats trouvés

### 5. Exécution de la recherche (`src/handler/search.rs`)

Implémenter `SearchAction::Execute` pour lancer la recherche dans les commits du graphe et stocker les résultats dans `search_state.results`. Naviguer automatiquement vers le premier résultat.

## Comparaison avec le filtre (qui fonctionne)

Le système de filtre (`F`) fonctionne car il a :
- Sa propre action `FilterInsertChar(char)` dans `AppAction`
- Son propre handler qui modifie `filter_popup.current_input_mut()`
- Son propre rendu dans `filter_popup.rs`

La recherche doit suivre le même pattern.

## Vérification

```bash
cargo build
# Tester : appuyer sur '/', taper du texte, vérifier l'affichage et la navigation dans les résultats
```
