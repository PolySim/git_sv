# STEP-06 : Amélioration des labels de branches et tags

## Priorité : MOYENNE (UX)

## Problèmes identifiés

### 6.1 — Pas de distinction visuelle entre branches locales, remotes et tags

Tous les refs sont affichés de la même façon : `[ref_name]` avec la même couleur (couleur de la branche du commit). On ne sait pas si `[main]` est une branche locale, `[origin/main]` est un remote, ou `[v1.0]` est un tag.

### Code actuel

```rust
// src/ui/graph_view.rs — build_commit_line()
if !node.refs.is_empty() {
    for ref_name in &node.refs {
        let ref_color = get_branch_color(node.color_index);
        spans.push(Span::styled(
            format!("[{}] ", ref_name),
            Style::default()
                .fg(ref_color)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED),
        ));
    }
}
```

### 6.2 — HEAD n'est pas identifié visuellement

La branche courante (HEAD) n'a pas d'indicateur spécial. Dans un graphe dense, il est difficile de repérer rapidement où on est.

### 6.3 — Les refs utilisent `REVERSED` qui est visuellement lourd

Le style `REVERSED` (inversion fond/texte) est très agressif visuellement. Sur certains terminaux, ça rend le texte difficile à lire.

## Fichiers impactés

| Fichier | Fonction |
|---------|----------|
| `src/ui/graph_view.rs` | `build_commit_line()` — section refs |
| `src/git/graph.rs` | `collect_refs()` — collecte des refs |
| `src/git/graph.rs` | `CommitNode` — structure (ajout potentiel d'info HEAD) |

## Solution proposée

### Étape 1 : Classifier les refs dans `collect_refs()`

Modifier `collect_refs()` pour distinguer les types de refs et indiquer HEAD :

```rust
// src/git/graph.rs

/// Type de référence.
#[derive(Debug, Clone, PartialEq)]
pub enum RefType {
    /// Branche locale.
    LocalBranch,
    /// Branche remote (origin/main, etc.).
    RemoteBranch,
    /// Tag.
    Tag,
    /// HEAD détaché ou HEAD pointant vers cette branche.
    Head,
}

/// Référence enrichie.
#[derive(Debug, Clone)]
pub struct RefInfo {
    pub name: String,
    pub ref_type: RefType,
}

// Dans CommitNode, remplacer :
// pub refs: Vec<String>,
// par :
// pub refs: Vec<RefInfo>,
```

Adapter `collect_refs()` :

```rust
fn collect_refs(repo: &Repository) -> Result<HashMap<Oid, Vec<RefInfo>>> {
    let mut map: HashMap<Oid, Vec<RefInfo>> = HashMap::new();
    
    // Déterminer HEAD
    let head_oid = repo.head().ok().and_then(|h| h.target());
    let head_branch = repo.head().ok().and_then(|h| {
        if h.is_branch() {
            h.shorthand().map(|s| s.to_string())
        } else {
            None
        }
    });

    for reference in repo.references()? {
        let reference = reference?;
        if let Some(name) = reference.shorthand() {
            if let Some(oid) = reference.target() {
                let ref_type = if name == "HEAD" {
                    continue; // On gère HEAD séparément
                } else if reference.is_tag() || name.starts_with("v") {
                    RefType::Tag
                } else if reference.is_remote() || name.contains('/') {
                    RefType::RemoteBranch
                } else {
                    RefType::LocalBranch
                };

                map.entry(oid)
                    .or_default()
                    .push(RefInfo {
                        name: name.to_string(),
                        ref_type,
                    });
            }
        }
    }

    // Marquer HEAD
    if let Some(oid) = head_oid {
        if let Some(branch) = &head_branch {
            // HEAD pointe vers une branche — marquer cette branche
            if let Some(refs) = map.get_mut(&oid) {
                for r in refs.iter_mut() {
                    if r.name == *branch {
                        r.ref_type = RefType::Head;
                    }
                }
            }
        }
    }

    Ok(map)
}
```

### Étape 2 : Style différencié dans `build_commit_line()`

```rust
// src/ui/graph_view.rs — build_commit_line()

if !node.refs.is_empty() {
    for ref_info in &node.refs {
        let (prefix, style) = match ref_info.ref_type {
            RefType::Head => {
                // HEAD : mise en avant forte
                ("", Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED))
            }
            RefType::LocalBranch => {
                // Branche locale : fond coloré
                ("", Style::default()
                    .fg(get_branch_color(node.color_index))
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED))
            }
            RefType::RemoteBranch => {
                // Remote : style plus discret, pas de REVERSED
                ("", Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM))
            }
            RefType::Tag => {
                // Tag : jaune, encadré différemment
                ("🏷 ", Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD))
            }
        };

        let bracket = match ref_info.ref_type {
            RefType::Head => format!("⦗{}⦘ ", ref_info.name),       // Crochets spéciaux
            RefType::Tag => format!("({})", ref_info.name),          // Parenthèses pour tags
            RefType::RemoteBranch => format!("⟨{}⟩ ", ref_info.name), // Chevrons pour remotes
            RefType::LocalBranch => format!("[{}] ", ref_info.name), // Crochets pour locales
        };

        spans.push(Span::styled(format!("{}{}", prefix, bracket), style));
    }
}
```

### Étape 3 : Trier les refs par pertinence

Afficher dans l'ordre : HEAD > branches locales > tags > remotes.

```rust
// Trier les refs avant l'affichage
let mut sorted_refs = node.refs.clone();
sorted_refs.sort_by_key(|r| match r.ref_type {
    RefType::Head => 0,
    RefType::LocalBranch => 1,
    RefType::Tag => 2,
    RefType::RemoteBranch => 3,
});
```

## Résultat visuel attendu

```
● abc1234 ⦗main⦘ [feature] ⟨origin/main⟩ (v1.0) Fix: correction du bug — Alice   il y a 2h
│ ●  def5678 [feature]                              Ajout du filtrage    — Bob     il y a 1h
```

- `⦗main⦘` en vert gras inversé (HEAD)
- `[feature]` en couleur de branche, gras inversé
- `⟨origin/main⟩` en gris discret
- `(v1.0)` en jaune gras

## Migration

Ce changement modifie la structure `CommitNode.refs` de `Vec<String>` vers `Vec<RefInfo>`. Il faudra adapter :
- `src/ui/detail_view.rs` — affichage des refs dans le panneau de détail
- `src/ui/graph_legend.rs` — légende des branches
- `src/state/mod.rs` — partout où `refs` est accédé
- Les tests existants

## Tests à ajouter

```rust
#[test]
fn test_ref_classification() {
    let (_temp, repo) = create_test_repo();
    let oid = commit_file(&repo, "test.txt", "content", "Initial");
    repo.branch("feature", &repo.find_commit(oid).unwrap(), false).unwrap();
    repo.tag_lightweight("v1.0", &repo.find_commit(oid).unwrap().into_object(), false).unwrap();
    
    let refs = collect_refs(&repo).unwrap();
    let commit_refs = refs.get(&oid).unwrap();
    
    assert!(commit_refs.iter().any(|r| r.ref_type == RefType::Head));
    assert!(commit_refs.iter().any(|r| r.ref_type == RefType::LocalBranch && r.name == "feature"));
    assert!(commit_refs.iter().any(|r| r.ref_type == RefType::Tag && r.name == "v1.0"));
}
```

## Critère de validation

- HEAD est immédiatement identifiable visuellement (vert gras).
- Les branches locales, remotes et tags sont visuellement distincts.
- Les remotes ne polluent pas visuellement le graphe.
- `cargo test` passe.
- `cargo clippy` OK.
