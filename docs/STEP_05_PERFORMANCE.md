# Phase 5 — Performance

## 5.1 Rafraîchissement trop fréquent

**Problème** : `refresh()` reconstruit intégralement le graphe (200 commits + parcours de toutes les refs) à chaque action qui nécessite une mise à jour. Certaines actions comme `SwitchToGraph` appellent `refresh()` même quand rien n'a changé.

- [ ] **Rafraîchissement conditionnel** : Ajouter un flag `dirty` qui indique si les données ont réellement changé (commit, stage, checkout, etc.) avant de relancer `refresh()`.
- [ ] **Rafraîchissement partiel** : Séparer `refresh_graph()`, `refresh_status()`, `refresh_branches()` pour ne recharger que ce qui est nécessaire.

## 5.2 Pas de cache pour les diffs

**Problème** : Chaque navigation dans la liste de fichiers recalcule le diff du fichier sélectionné, même s'il a déjà été calculé.

- [ ] Implémenter un cache LRU simple pour les diffs de fichiers (ex: `HashMap<(Oid, String), FileDiff>` avec une taille max).

## 5.3 Construction du graphe coûteuse

**Problème** : `build_graph()` itère sur toutes les refs à chaque appel pour collecter les refs map.

- [ ] **Lazy loading** : Ne charger que les N premiers commits visibles, puis charger plus à la demande (pagination).
- [ ] **Caching des refs** : Ne recalculer la refs map que si une opération git a eu lieu.

## 5.4 Tick rate et polling

**Problème** : Le polling d'événements est fait à 100ms, ce qui signifie que même sans activité, le CPU est occupé.

- [ ] Augmenter le timeout de poll quand aucune animation n'est en cours.
- [ ] Utiliser un système d'événements basé sur des channels (cf. pattern ratatui recommandé) pour séparer le thread d'événements du thread de rendu.
