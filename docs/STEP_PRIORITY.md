# Ordre de priorité recommandé

## Sprint 1 — Fondations (refactorisation critique)
1. **1.1** Découper `app.rs`
2. **1.2** Réduire la signature de `render()`
3. **2.1** Éliminer `centered_rect()` dupliqué
4. **2.2** Factoriser `branch.rs` (et corriger le double `graph_ahead_behind`)
5. **2.3** Factoriser `diff.rs`
6. **3.1** Unifier la gestion d'erreurs
7. **3.2** Corriger le double appel à `status()`
8. **3.5** Ajouter `Copy` aux enums simples

## Sprint 2 — Fiabilité
9. **3.3** Corriger la gestion Unicode dans InsertChar
10. **3.4** Nettoyage clippy complet
11. **4.1-4.9** Ajouter les tests (au moins git/, app state, CLI)
12. **7.1-7.3** Implémenter les TODO existants

## Sprint 3 — Fonctionnalités essentielles
13. **8.6** Discard de fichier
14. **8.1** Git Push / Pull / Fetch
15. **8.2** Recherche de commits
16. **8.5** Commit amend
17. **6.2** Confirmation pour actions destructives

## Sprint 4 — UX & Performance
18. **5.1** Rafraîchissement conditionnel
19. **5.2** Cache des diffs
20. **6.1** Dates relatives
21. **6.4** Support souris
22. **6.8** Barre de navigation entre vues
23. **5.4** Event loop optimisée (channels)

## Sprint 5 — Fonctionnalités avancées
24. **1.3** Restructurer les modules UI en sous-dossiers
25. **8.4** Cherry-pick
26. **8.7** Gestion des tags
27. **8.3** Vue Blame
28. **8.9** Diff entre branches
29. **6.7** Diff amélioré (side-by-side, syntax highlighting)

## Sprint 6 — Polish
30. **5.3** Lazy loading du graphe
31. **6.3** Indicateur de chargement
32. **6.5** Thèmes configurables
33. **8.10** Configuration / Préférences
34. **8.8** Rebase simplifié
35. **2.4** Status bar unifiée
36. **6.6** Légende du graphe

## Sprint 7 — Extras (si temps)
37. **8.11** Intégration forges (GitHub/GitLab)
38. **8.12** Export du graphe
