# STEP-004 — Feature : Afficher le nom de branche dans l'onglet détail

## Problème

Dans l'onglet détail d'un commit (vue Graph, panneau Detail), on ne voit pas de quelle branche provient le commit. Les informations affichées sont : hash, auteur, date, refs, parents, message. Il manque la branche source.

## Fichiers concernés

- `src/ui/detail_view.rs` — Rendu du panneau détail (l13-89)
- `src/git/graph.rs` — Structure `CommitNode` (contient `refs` et possiblement `branch_refs`)

## Analyse

Le `CommitNode` contient déjà un champ `refs` qui inclut les références (tags, branches) pointant sur ce commit. Cependant :
- `refs` ne contient que les refs qui pointent **exactement** sur ce commit (pas la branche parente dans le graphe)
- Pour les commits intermédiaires (qui ne sont pas la tête d'une branche), `refs` est vide

Ce qu'il faut, c'est la branche **de laquelle** provient le commit dans le graphe. Cette information est disponible dans le graphe via la colonne et la couleur du commit.

## Solution

### Étape 1 — Ajouter la branche source dans `CommitNode`

Dans `src/git/graph.rs`, lors de la construction du graphe, stocker la branche associée à chaque commit (la branche dont le commit fait partie dans le graphe). Cette info peut être déduite de la colonne du commit et des refs de la branche tête.

### Étape 2 — Afficher dans detail_view

Ajouter une ligne "Branche:" dans `src/ui/detail_view.rs` :

```rust
// Après les refs et parents existants
if let Some(branch) = &node.branch_name {
    lines.push(Line::from(vec![
        Span::styled("Branche: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(branch.clone(), Style::default().fg(Color::Green)),
    ]));
}
```

### Alternative simplifiée

Si la branche source est trop complexe à calculer (nécessite de tracer le graphe), afficher au minimum les refs existantes de manière plus visible, et pour les commits intermédiaires, afficher la branche qui contient ce commit via `git2` :

```rust
// Trouver les branches contenant ce commit
let branches = repo.branches(Some(BranchType::Local))?;
for branch in branches {
    let (branch, _) = branch?;
    if let Ok(true) = repo.graph_descendant_of(branch.get().peel_to_commit()?.id(), commit_oid) {
        // Ce commit est un ancêtre de cette branche
    }
}
```

## Tests

- Vérifier qu'un commit sur `main` affiche "Branche: main"
- Vérifier qu'un commit sur une feature branch affiche le bon nom
- Vérifier le comportement pour les commits de merge (plusieurs branches)
- Vérifier le comportement pour un commit sans branche (branche supprimée)
