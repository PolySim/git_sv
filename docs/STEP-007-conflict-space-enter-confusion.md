# STEP-007 : Résolution de conflit — La touche Espace agit comme Enter en mode Ligne

## Problème

En résolution de conflit en mode Ligne, appuyer sur Espace (pour toggler la sélection
d'une ligne) peut parfois valider le fichier entier et passer au suivant, comme si on
avait appuyé sur Enter.

**Cause racine :** Dans `handle_toggle_line()` (src/handler/conflict.rs, ligne ~648),
après chaque toggle d'une ligne, le code vérifie si **toutes les sections sont résolues**
via `all_sections_resolved(file)`. Si c'est le cas, il **applique automatiquement la
résolution sur disque et passe au fichier suivant** (lignes ~700-717).

Cela signifie que si le toggle de la dernière ligne non-sélectionnée complète la résolution
de toutes les sections, un simple Espace a le même effet qu'Enter — le fichier est finalisé
sans confirmation explicite de l'utilisateur.

### Flux problématique

```
1. Fichier avec 3 sections de conflit
2. L'utilisateur toggle les lignes des 2 premières sections
3. L'utilisateur toggle la dernière ligne de la 3ème section
4. all_sections_resolved() → true
5. → Résolution automatique appliquée sur disque
6. → Passage au fichier suivant
7. L'utilisateur n'a jamais appuyé sur Enter pour confirmer
```

### Bug secondaire dans `handle_enter_resolve`

La fonction `handle_enter_resolve()` (ligne ~981) contient aussi un cas pour `Line` mode
(lignes ~1014-1015) qui appelle `handle_toggle_line()`. Cela crée une confusion
supplémentaire où Enter en mode Line toggle au lieu de valider dans certaines conditions.

## Fichiers concernés

| Fichier | Rôle |
|---------|------|
| `src/handler/conflict.rs` | `handle_toggle_line()` (ligne ~648) — auto-apply quand tout est résolu |
| `src/handler/conflict.rs` | `handle_enter_resolve()` (ligne ~981) — délègue à toggle_line en mode Line |
| `src/handler/conflict.rs` | `handle_mark_resolved()` (ligne ~492) — écriture sur disque et passage au suivant |
| `src/ui/input.rs` | Mapping Space → `ConflictToggleLine` (ligne ~535) et Enter → `ConflictResolveFile` (ligne ~549) |
| `src/git/conflict.rs` | `all_sections_resolved()`, `apply_resolved_content()` |

## Plan de correction

### 1. Supprimer l'auto-apply dans `handle_toggle_line()`

Dans `src/handler/conflict.rs`, `handle_toggle_line()` (lignes ~700-717) :

```rust
// SUPPRIMER ce bloc :
if all_sections_resolved(file) {
    // application automatique + passage au fichier suivant
    ...
}
```

Le toggle ne doit **que** changer l'état de sélection de la ligne. La validation doit
rester explicite via Enter (`ConflictResolveFile` → `handle_mark_resolved`).

### 2. Optionnel : indicateur visuel quand tout est résolu

Au lieu d'appliquer automatiquement, on peut afficher un indicateur visuel (ex: titre
du fichier en vert, message dans la barre d'aide) pour indiquer que toutes les sections
sont résolues et que l'utilisateur peut appuyer sur Enter pour valider.

### 3. Nettoyer `handle_enter_resolve` pour le mode Line

Le match arm pour `Line` dans `handle_enter_resolve` (ligne ~1014) ne devrait pas appeler
`handle_toggle_line()`. Il devrait soit :
- Ne rien faire (car Enter en mode Line est déjà mappé à `ConflictResolveFile`)
- Rediriger vers `handle_mark_resolved()` si on veut qu'Enter résolve le fichier

### 4. Vérification

- [ ] En mode Ligne, Espace toggle la sélection d'une ligne sans jamais valider le fichier
- [ ] Même quand toutes les lignes sont sélectionnées, Espace ne finalise pas automatiquement
- [ ] Enter reste le seul moyen de valider/écrire la résolution sur disque
- [ ] Le workflow complet : toggle lignes → Enter → fichier suivant fonctionne correctement
- [ ] Aucune régression en mode Block et File
