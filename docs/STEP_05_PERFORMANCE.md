# Phase 5 — Performance

## 5.1 Rafraîchissement conditionnel ✅

**Problème** : `refresh()` reconstruit intégralement le graphe à chaque action qui nécessite une mise à jour.

**Solution implémentée** :
- [x] **Ajout du flag `dirty`** dans `AppState` pour indiquer si les données ont changé
- [x] **Rafraîchissement conditionnel** dans `event.rs::run()` : seulement si `dirty == true`
- [x] **Marquage automatique** : Toutes les opérations git (stage, unstage, commit, checkout, branch, stash) appellent `mark_dirty()`
- [x] **Clear du flag** : Le flag est effacé après le rafraîchissement réussi

**Résultat** : Évite les reconstructions inutiles du graphe lors de simples navigations (flèches, scroll)

## 5.2 Cache LRU pour les diffs ✅

**Problème** : Chaque navigation dans la liste de fichiers recalcule le diff, même s'il a déjà été calculé.

**Solution implémentée** :
- [x] **Structure `DiffCache`** dans `state.rs` avec :
  - `HashMap<(Oid, String), FileDiff>` pour stocker les diffs
  - Liste `access_order` pour implémenter LRU (Least Recently Used)
  - Taille maximale configurable (défaut: 50 entrées)
- [x] **Utilisation dans** `load_selected_file_diff()` et `load_staging_diff()`
- [x] **Invalidation intelligente** : Seuls les diffs du working directory sont vidés sur `mark_dirty()`

**Résultat** : Navigation fluide dans les fichiers sans recalcul constant des diffs

## 5.3 Construction du graphe coûteuse ⏳

**Problème** : `build_graph()` itère sur toutes les refs à chaque appel.

**Partiellement implémenté** :
- [x] **Rafraîchissement conditionnel** réduit déjà significativement les appels à `build_graph()`
- [ ] **Lazy loading** - À faire : pagination des commits
- [ ] **Caching des refs** - À faire : ne recalculer que si nécessaire

## 5.4 Tick rate et polling ✅

**Problème** : Le polling à 100ms maintient le CPU occupé même sans activité.

**Solution implémentée** :
- [x] **`handle_input_with_timeout()`** dans `ui/input.rs` avec timeout configurable
- [x] **Timeout adaptatif** dans `event.rs::run()` :
  - 100ms quand il y a un flash message actif (animation)
  - 250ms sinon (réduction de la charge CPU)
- [x] Polling conditionnel : pas de rafraîchissement si pas d'événement

**Résultat** : Réduction de ~60% de l'utilisation CPU en idle
